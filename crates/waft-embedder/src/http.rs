//! HTTP as the host's: a request sent, its response head once that arrives, and its body
//! streamed as it comes. The values are the `http` crate's, so a request or a response is the
//! same shape whichever platform's client sent it; only [`Body`] is Waft's, because a body that
//! streams is a future this framework drives rather than a buffer it already holds.
//!
//! A platform answers [`Platform::http`](crate::Platform::http) with its client, or `None` where
//! it has none. The client is `NSURLSession` on Apple's systems, `fetch` in a browser, and the
//! engine's own asynchronous HTTP on a WebAssembly component; a host without one leaves HTTP to
//! the app.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use http::{HeaderMap, Request, Response};

/// A request on its way to a server, and the response once its head arrives.
///
/// The response is ready before its body is: the body streams from [`Response::body_mut`], and
/// dropping the future or the response before the body is read cancels the transfer.
pub type HttpFuture = Pin<Box<dyn Future<Output = Result<Response<Body>, HttpError>>>>;

/// Sends HTTP requests. One client outlives many requests, and every request may be in flight at
/// once.
pub trait HttpClient {
    /// Sends `request`. The answer arrives once the response's head has, with its body still to
    /// read; dropping the future before then cancels the request.
    fn send(&self, request: Request<Body>, options: SendOptions) -> HttpFuture;
}

/// How a request follows redirects and how long it may take.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendOptions {
    /// Which redirects the client follows.
    pub redirects: RedirectPolicy,
    /// How long the whole exchange may take, or `None` for the client's own limit. A client
    /// reports [`HttpError::Timeout`] when it runs out.
    pub timeout: Option<Duration>,
}

impl SendOptions {
    /// Follow up to five redirects, and wait as long as the platform does.
    pub const fn new() -> SendOptions {
        SendOptions {
            redirects: RedirectPolicy::Follow { max: 5 },
            timeout: None,
        }
    }

    /// Dart `HttpClient.maxRedirects = max`: how many redirects to follow.
    pub fn redirects(mut self, redirects: RedirectPolicy) -> SendOptions {
        self.redirects = redirects;
        self
    }

    /// Dart `HttpClient.connectionTimeout`: how long the exchange may take.
    pub fn timeout(mut self, timeout: Duration) -> SendOptions {
        self.timeout = Some(timeout);
        self
    }
}

impl Default for SendOptions {
    fn default() -> SendOptions {
        SendOptions::new()
    }
}

/// What a client does with a redirect: follows it, up to a number, or answers the redirect
/// response itself.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RedirectPolicy {
    /// Follow up to `max` redirects, then fail with [`HttpError::TooManyRedirects`]. Dart's
    /// `HttpClient` follows five by default.
    Follow {
        /// How many redirects may be followed.
        max: usize,
    },
    /// Answer the first redirect response as it is, as Dart's `followRedirects = false` does.
    #[default]
    Never,
}

/// Why a request got no response, or its body stopped short.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HttpError {
    /// The URI is not one a client can send to: it has no scheme, or no host.
    InvalidUrl,
    /// The server refused the connection.
    Refused(String),
    /// The connection failed, or ended before the body did.
    Connection(String),
    /// TLS refused the server's certificate, or the handshake failed.
    Tls(String),
    /// The exchange took longer than its timeout.
    Timeout,
    /// A redirect policy allowed fewer redirects than the server sent.
    TooManyRedirects,
    /// The server answered something that is not HTTP, or the client could not read it.
    Protocol(String),
}

impl fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::InvalidUrl => {
                formatter.write_str("the URL is not one an HTTP client can send to")
            }
            HttpError::Refused(reason) => write!(formatter, "the connection was refused: {reason}"),
            HttpError::Connection(reason) => write!(formatter, "the connection failed: {reason}"),
            HttpError::Tls(reason) => write!(formatter, "TLS failed: {reason}"),
            HttpError::Timeout => formatter.write_str("the request timed out"),
            HttpError::TooManyRedirects => formatter.write_str("too many redirects"),
            HttpError::Protocol(reason) => write!(formatter, "HTTP failed: {reason}"),
        }
    }
}

impl std::error::Error for HttpError {}

/// A body on its way to or from a server: bytes read a piece at a time, the length when the
/// sender said it, and trailers once the body has ended. Read as [`FileStream`] is.
///
/// [`FileStream`]: crate::FileStream
pub trait BodyStream {
    /// Reads the next bytes into `buf`, answering how many it read, `0` once the body has ended.
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a>;

    /// The fields that came after the body, once it has ended; `None` where the response has
    /// none, and always `None` on a client whose protocol layer does not expose them.
    fn trailers(&mut self) -> BodyTrailersFuture<'_>;

    /// How many bytes the whole body carries, when the sender said so, as `Content-Length`
    /// does. Undefined when the body is compressed or sent in pieces.
    fn length(&self) -> Option<u64>;
}

/// A [`BodyStream::read`] answer: bytes read, or why the body stopped short.
pub type BodyReadFuture<'a> = Pin<Box<dyn Future<Output = Result<usize, HttpError>> + 'a>>;

/// A [`BodyStream::trailers`] answer: the trailers, or why the body stopped short.
pub type BodyTrailersFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Option<HeaderMap>, HttpError>> + 'a>>;

/// A [`Body::read_all`] answer: the body, whole.
pub type BodyReadAllFuture<'a> = Pin<Box<dyn Future<Output = Result<Vec<u8>, HttpError>> + 'a>>;

/// The body of a request or a response. It is empty, bytes already in memory, or a stream read
/// as fast as the platform delivers it.
pub struct Body(Option<Box<dyn BodyStream>>);

/// How much one whole read asks for at a time.
const READ_CHUNK: usize = 64 * 1024;

impl Body {
    /// No bytes at all: a `GET`'s body, or a response with nothing in it.
    pub fn empty() -> Body {
        Body(None)
    }

    /// The bytes `stream` reads, as they come.
    pub fn from_stream(stream: impl BodyStream + 'static) -> Body {
        Body(Some(Box::new(stream)))
    }

    /// Reads the next bytes of the body into `buf`, answering how many it read, `0` at the end:
    /// a piece at a time, as the platform delivers it.
    pub fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        match &mut self.0 {
            Some(stream) => stream.read(buf),
            None => Box::pin(std::future::ready(Ok(0))),
        }
    }

    /// The whole body, read through: the bytes a request carries, or a response's.
    pub fn read_all(&mut self) -> BodyReadAllFuture<'_> {
        let stream: Option<&mut (dyn BodyStream + '_)> = match &mut self.0 {
            Some(stream) => Some(&mut **stream),
            None => None,
        };
        Box::pin(read_all(stream))
    }

    /// How many bytes the whole body carries, when its sender said so. `None` for an empty body
    /// and for one whose length is unknown.
    pub fn length(&self) -> Option<u64> {
        self.0.as_ref().and_then(|stream| stream.length())
    }
}

impl From<Vec<u8>> for Body {
    fn from(bytes: Vec<u8>) -> Body {
        Body(Some(Box::new(BufferedBody::new(bytes))))
    }
}

impl From<&'static [u8]> for Body {
    fn from(bytes: &'static [u8]) -> Body {
        Body(Some(Box::new(BufferedBody::new(bytes.to_vec()))))
    }
}

impl Default for Body {
    fn default() -> Body {
        Body::empty()
    }
}

impl fmt::Debug for Body {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            None => formatter.write_str("Body(empty)"),
            Some(stream) => formatter
                .debug_struct("Body")
                .field("length", &stream.length())
                .finish(),
        }
    }
}

async fn read_all(stream: Option<&mut dyn BodyStream>) -> Result<Vec<u8>, HttpError> {
    let Some(stream) = stream else {
        return Ok(Vec::new());
    };
    let mut bytes = Vec::with_capacity(usize::try_from(stream.length().unwrap_or(0)).unwrap_or(0));
    let mut chunk = vec![0; READ_CHUNK];
    loop {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            return Ok(bytes);
        }
        bytes.extend_from_slice(&chunk[..read]);
    }
}

/// Bytes already in memory, read at once.
struct BufferedBody {
    bytes: Vec<u8>,
    read: usize,
}

impl BufferedBody {
    fn new(bytes: Vec<u8>) -> BufferedBody {
        BufferedBody { bytes, read: 0 }
    }
}

impl BodyStream for BufferedBody {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        let read = (self.bytes.len() - self.read).min(buf.len());
        buf[..read].copy_from_slice(&self.bytes[self.read..self.read + read]);
        self.read += read;
        Box::pin(std::future::ready(Ok(read)))
    }

    fn trailers(&mut self) -> BodyTrailersFuture<'_> {
        Box::pin(std::future::ready(Ok(None)))
    }

    fn length(&self) -> Option<u64> {
        Some(self.bytes.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let std::task::Poll::Ready(output) = future.as_mut().poll(&mut context) {
                return output;
            }
        }
    }

    #[test]
    fn an_empty_body_reads_nothing() {
        let mut body = Body::empty();
        assert_eq!(now(body.read_all()).unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn a_body_of_bytes_reads_whole_and_says_its_length() {
        let mut body = Body::from(b"hello, body".to_vec());
        assert_eq!(body.0.as_ref().unwrap().length(), Some(11));
        assert_eq!(now(body.read_all()).unwrap(), b"hello, body");
    }

    #[test]
    fn a_streamed_body_is_read_a_piece_at_a_time() {
        struct Pieces(Vec<u8>, usize);

        impl BodyStream for Pieces {
            fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
                let read = (self.0.len() - self.1).min(buf.len()).min(3);
                buf[..read].copy_from_slice(&self.0[self.1..self.1 + read]);
                self.1 += read;
                Box::pin(std::future::ready(Ok(read)))
            }

            fn trailers(&mut self) -> BodyTrailersFuture<'_> {
                let mut trailers = HeaderMap::new();
                trailers.insert("x-checksum", 7u32.into());
                Box::pin(std::future::ready(Ok(Some(trailers))))
            }

            fn length(&self) -> Option<u64> {
                Some(self.0.len() as u64)
            }
        }

        let mut body = Body::from_stream(Pieces(b"streamed body".to_vec(), 0));
        assert_eq!(now(body.read_all()).unwrap(), b"streamed body");
        let mut stream = body.0.take().unwrap();
        assert_eq!(now(stream.trailers()).unwrap().unwrap()["x-checksum"], "7");
    }

    #[test]
    fn send_options_default_to_five_redirects_and_the_clients_own_limit() {
        let options = SendOptions::default();
        assert_eq!(options.redirects, RedirectPolicy::Follow { max: 5 });
        assert_eq!(options.timeout, None);
        let options = SendOptions::new()
            .redirects(RedirectPolicy::Never)
            .timeout(Duration::from_secs(2));
        assert_eq!(options.redirects, RedirectPolicy::Never);
        assert_eq!(options.timeout, Some(Duration::from_secs(2)));
    }

    #[test]
    fn every_error_says_what_went_wrong() {
        assert_eq!(
            HttpError::TooManyRedirects.to_string(),
            "too many redirects"
        );
        assert_eq!(HttpError::Timeout.to_string(), "the request timed out");
    }
}
