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

/// How a request follows redirects and how long each step of it may take.
///
/// The three timeouts are WASI 0.3's `request-options`: how long connecting may take, how long
/// the first byte of the response body may take after the head, and how long the gap between
/// bytes of the body may take. Each `None` means no limit, as WASI's does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SendOptions {
    /// Which redirects the client follows.
    pub redirects: RedirectPolicy,
    /// WASI `connect-timeout`: how long the initial connection to the server may take.
    pub connect_timeout: Option<Duration>,
    /// WASI `first-byte-timeout`: how long the first byte of the response body may take after
    /// the response head arrived.
    pub first_byte_timeout: Option<Duration>,
    /// WASI `between-bytes-timeout`: how long the gap between one piece of the response body and
    /// the next may take.
    pub between_bytes_timeout: Option<Duration>,
}

impl SendOptions {
    /// Follow up to five redirects, and let every step take as long as the platform does.
    pub const fn new() -> SendOptions {
        SendOptions {
            redirects: RedirectPolicy::DEFAULT,
            connect_timeout: None,
            first_byte_timeout: None,
            between_bytes_timeout: None,
        }
    }

    /// Dart `HttpClient.maxRedirects = max`: how many redirects to follow.
    pub fn redirects(mut self, redirects: RedirectPolicy) -> SendOptions {
        self.redirects = redirects;
        self
    }

    /// WASI `connect-timeout`: how long the initial connection may take.
    pub fn connect_timeout(mut self, timeout: Duration) -> SendOptions {
        self.connect_timeout = Some(timeout);
        self
    }

    /// WASI `first-byte-timeout`: how long the first byte of the response body may take.
    pub fn first_byte_timeout(mut self, timeout: Duration) -> SendOptions {
        self.first_byte_timeout = Some(timeout);
        self
    }

    /// WASI `between-bytes-timeout`: how long the gap between two pieces of the response body
    /// may take.
    pub fn between_bytes_timeout(mut self, timeout: Duration) -> SendOptions {
        self.between_bytes_timeout = Some(timeout);
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RedirectPolicy {
    /// Follow up to `max` redirects, then fail with [`HttpError::TooManyRedirects`]. Dart's
    /// `HttpClient` follows five by default.
    Follow {
        /// How many redirects may be followed.
        max: usize,
    },
    /// Answer the first redirect response as it is, as Dart's `followRedirects = false` does.
    Never,
}

impl RedirectPolicy {
    /// Dart's `HttpClient`: follow up to five redirects.
    pub const DEFAULT: RedirectPolicy = RedirectPolicy::Follow { max: 5 };
}

impl Default for RedirectPolicy {
    fn default() -> RedirectPolicy {
        RedirectPolicy::DEFAULT
    }
}

/// Why a request got no response, or its body stopped short.
///
/// WASI 0.3's `error-code` cases, Rust-named, so a WebAssembly component's adapter passes them
/// through and every other platform maps its own failures into them; [`TooManyRedirects`] is
/// Waft's, the one thing WASI leaves to the application.
///
/// [`TooManyRedirects`]: HttpError::TooManyRedirects
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HttpError {
    /// The DNS lookup did not finish in the time the platform allowed.
    DnsTimeout,
    /// The DNS lookup failed: `rcode` is the resolver's response code when it gave one, and
    /// `info_code` its extended information code.
    DnsError(DnsErrorPayload),
    /// The destination host or path was not found.
    DestinationNotFound,
    /// The destination is temporarily unavailable.
    DestinationUnavailable,
    /// The destination's address is one the platform's policy forbids.
    DestinationIpProhibited,
    /// The destination's address cannot be routed from this network.
    DestinationIpUnroutable,
    /// The server refused the connection.
    ConnectionRefused,
    /// The connection ended before the exchange did.
    ConnectionTerminated,
    /// Connecting to the server took longer than [`SendOptions::connect_timeout`].
    ConnectionTimeout,
    /// Reading from the connection took longer than the platform allows.
    ConnectionReadTimeout,
    /// Writing to the connection took longer than the platform allows.
    ConnectionWriteTimeout,
    /// The platform's limit on open connections was reached.
    ConnectionLimitReached,
    /// TLS failed: the handshake or one of its records was not valid TLS.
    TlsProtocolError,
    /// TLS refused the server's certificate, or the server refused the client's.
    TlsCertificateError,
    /// The peer sent a TLS alert: `alert_id` is the alert's number when it gave one, and
    /// `alert_message` its description.
    TlsAlertReceived(TlsAlertReceivedPayload),
    /// The platform's policy does not allow the request.
    HttpRequestDenied,
    /// The request must say its length, and does not.
    HttpRequestLengthRequired,
    /// The request body is larger than the platform allows; the limit, when it names one.
    HttpRequestBodySize(Option<u64>),
    /// The method is not a valid HTTP method.
    HttpRequestMethodInvalid,
    /// The URI is not one the client can send to.
    HttpRequestUriInvalid,
    /// The URI is longer than the platform allows.
    HttpRequestUriTooLong,
    /// The request's header section is larger than the platform allows; the limit, when it
    /// names one.
    HttpRequestHeaderSectionSize(Option<u32>),
    /// One of the request's header fields is too large: which field, and the limit, when the
    /// platform names them.
    HttpRequestHeaderSize(Option<FieldSizePayload>),
    /// The request's trailer section is larger than the platform allows; the limit, when it
    /// names one.
    HttpRequestTrailerSectionSize(Option<u32>),
    /// One of the request's trailer fields is too large.
    HttpRequestTrailerSize(FieldSizePayload),
    /// The response ended before its head or body was whole.
    HttpResponseIncomplete,
    /// The response's header section is larger than the platform allows; the limit, when it
    /// names one.
    HttpResponseHeaderSectionSize(Option<u32>),
    /// One of the response's header fields is too large.
    HttpResponseHeaderSize(FieldSizePayload),
    /// The response body is larger than the platform allows; the limit, when it names one.
    HttpResponseBodySize(Option<u64>),
    /// The response's trailer section is larger than the platform allows; the limit, when it
    /// names one.
    HttpResponseTrailerSectionSize(Option<u32>),
    /// One of the response's trailer fields is too large.
    HttpResponseTrailerSize(FieldSizePayload),
    /// The response's transfer coding is one the platform does not take; its name, when known.
    HttpResponseTransferCoding(Option<String>),
    /// The response's content coding is one the platform does not take; its name, when known.
    HttpResponseContentCoding(Option<String>),
    /// The response did not arrive in time: its first body byte after the head, or the next one
    /// after the last, took longer than [`SendOptions::first_byte_timeout`] or
    /// [`SendOptions::between_bytes_timeout`] allowed.
    HttpResponseTimeout,
    /// The request's protocol upgrade failed.
    HttpUpgradeFailed,
    /// The server spoke something that is not valid HTTP.
    HttpProtocolError,
    /// A proxy on the way detected a loop.
    LoopDetected,
    /// The platform's HTTP implementation is not configured to send this request.
    ConfigurationError,
    /// A failure that fits no other case, with an unstructured description when one is known;
    /// WASI warns that the string need not mean the same between implementations.
    Internal(Option<String>),
    /// A redirect policy allowed fewer redirects than the server sent. Not a WASI case: WASI
    /// leaves the redirect limit to the application.
    TooManyRedirects,
}

/// The payload of [`HttpError::DnsError`], as WASI's `DNS-error-payload`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DnsErrorPayload {
    /// The resolver's response code, when it gave one.
    pub rcode: Option<String>,
    /// The resolver's extended information code, when it gave one.
    pub info_code: Option<u16>,
}

/// The payload of [`HttpError::TlsAlertReceived`], as WASI's `TLS-alert-received-payload`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TlsAlertReceivedPayload {
    /// The alert's number, when the peer gave one.
    pub alert_id: Option<u8>,
    /// The alert's description, when the peer gave one.
    pub alert_message: Option<String>,
}

/// A header or trailer field too large for the platform, as WASI's `field-size-payload`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSizePayload {
    /// The field's name, when the platform names it.
    pub field_name: Option<String>,
    /// The field's size, when the platform names it.
    pub field_size: Option<u32>,
}

impl fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::DnsTimeout => formatter.write_str("the DNS lookup timed out"),
            HttpError::DnsError(payload) => write!(
                formatter,
                "the DNS lookup failed: {}{}",
                payload.rcode.as_deref().unwrap_or("no response code"),
                payload
                    .info_code
                    .map(|code| format!(" (info code {code})"))
                    .unwrap_or_default()
            ),
            HttpError::DestinationNotFound => formatter.write_str("the destination was not found"),
            HttpError::DestinationUnavailable => {
                formatter.write_str("the destination is unavailable")
            }
            HttpError::DestinationIpProhibited => {
                formatter.write_str("the destination's address is prohibited")
            }
            HttpError::DestinationIpUnroutable => {
                formatter.write_str("the destination's address is unroutable")
            }
            HttpError::ConnectionRefused => formatter.write_str("the connection was refused"),
            HttpError::ConnectionTerminated => formatter.write_str("the connection terminated"),
            HttpError::ConnectionTimeout => formatter.write_str("the connection timed out"),
            HttpError::ConnectionReadTimeout => {
                formatter.write_str("reading from the connection timed out")
            }
            HttpError::ConnectionWriteTimeout => {
                formatter.write_str("writing to the connection timed out")
            }
            HttpError::ConnectionLimitReached => {
                formatter.write_str("the connection limit was reached")
            }
            HttpError::TlsProtocolError => formatter.write_str("TLS failed"),
            HttpError::TlsCertificateError => {
                formatter.write_str("the TLS certificate was refused")
            }
            HttpError::TlsAlertReceived(payload) => write!(
                formatter,
                "a TLS alert was received: {}",
                payload.alert_message.as_deref().unwrap_or("no description")
            ),
            HttpError::HttpRequestDenied => formatter.write_str("the request was denied"),
            HttpError::HttpRequestLengthRequired => {
                formatter.write_str("the request must say its length")
            }
            HttpError::HttpRequestBodySize(limit) => write!(
                formatter,
                "the request body is too large{}",
                limit
                    .map(|limit| format!(" (limit {limit})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpRequestMethodInvalid => formatter.write_str("the method is not valid"),
            HttpError::HttpRequestUriInvalid => formatter.write_str("the URI is not valid"),
            HttpError::HttpRequestUriTooLong => formatter.write_str("the URI is too long"),
            HttpError::HttpRequestHeaderSectionSize(limit) => write!(
                formatter,
                "the request's header section is too large{}",
                limit
                    .map(|limit| format!(" (limit {limit})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpRequestHeaderSize(field) => write_field_size(
                formatter,
                "a request header field is too large",
                field.as_ref(),
            ),
            HttpError::HttpRequestTrailerSectionSize(limit) => write!(
                formatter,
                "the request's trailer section is too large{}",
                limit
                    .map(|limit| format!(" (limit {limit})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpRequestTrailerSize(field) => write_field_size(
                formatter,
                "a request trailer field is too large",
                Some(field),
            ),
            HttpError::HttpResponseIncomplete => formatter.write_str("the response is incomplete"),
            HttpError::HttpResponseHeaderSectionSize(limit) => write!(
                formatter,
                "the response's header section is too large{}",
                limit
                    .map(|limit| format!(" (limit {limit})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpResponseHeaderSize(field) => write_field_size(
                formatter,
                "a response header field is too large",
                Some(field),
            ),
            HttpError::HttpResponseBodySize(limit) => write!(
                formatter,
                "the response body is too large{}",
                limit
                    .map(|limit| format!(" (limit {limit})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpResponseTrailerSectionSize(limit) => write!(
                formatter,
                "the response's trailer section is too large{}",
                limit
                    .map(|limit| format!(" (limit {limit})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpResponseTrailerSize(field) => write_field_size(
                formatter,
                "a response trailer field is too large",
                Some(field),
            ),
            HttpError::HttpResponseTransferCoding(coding) => write!(
                formatter,
                "the response's transfer coding is not accepted{}",
                coding
                    .as_ref()
                    .map(|coding| format!(" ({coding})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpResponseContentCoding(coding) => write!(
                formatter,
                "the response's content coding is not accepted{}",
                coding
                    .as_ref()
                    .map(|coding| format!(" ({coding})"))
                    .unwrap_or_default()
            ),
            HttpError::HttpResponseTimeout => formatter.write_str("the response timed out"),
            HttpError::HttpUpgradeFailed => formatter.write_str("the protocol upgrade failed"),
            HttpError::HttpProtocolError => formatter.write_str("the server spoke invalid HTTP"),
            HttpError::LoopDetected => formatter.write_str("a proxy detected a loop"),
            HttpError::ConfigurationError => {
                formatter.write_str("the HTTP client is not configured for this request")
            }
            HttpError::Internal(reason) => match reason {
                Some(reason) => write!(formatter, "HTTP failed: {reason}"),
                None => formatter.write_str("HTTP failed"),
            },
            HttpError::TooManyRedirects => formatter.write_str("too many redirects"),
        }
    }
}

fn write_field_size(
    formatter: &mut fmt::Formatter<'_>,
    what: &str,
    field: Option<&FieldSizePayload>,
) -> fmt::Result {
    match field.and_then(|field| field.field_name.as_deref()) {
        Some(name) => write!(formatter, "{what}: {name}"),
        None => formatter.write_str(what),
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
    fn send_options_default_to_five_redirects_and_no_timeouts() {
        let options = SendOptions::default();
        assert_eq!(options.redirects, RedirectPolicy::Follow { max: 5 });
        assert_eq!(options.connect_timeout, None);
        assert_eq!(options.first_byte_timeout, None);
        assert_eq!(options.between_bytes_timeout, None);
        assert_eq!(RedirectPolicy::default(), options.redirects);
        let options = SendOptions::new()
            .redirects(RedirectPolicy::Never)
            .connect_timeout(Duration::from_secs(1))
            .first_byte_timeout(Duration::from_secs(2))
            .between_bytes_timeout(Duration::from_secs(3));
        assert_eq!(options.redirects, RedirectPolicy::Never);
        assert_eq!(options.connect_timeout, Some(Duration::from_secs(1)));
        assert_eq!(options.first_byte_timeout, Some(Duration::from_secs(2)));
        assert_eq!(options.between_bytes_timeout, Some(Duration::from_secs(3)));
    }

    #[test]
    fn every_error_says_what_went_wrong() {
        assert_eq!(
            HttpError::TooManyRedirects.to_string(),
            "too many redirects"
        );
        assert_eq!(
            HttpError::ConnectionTimeout.to_string(),
            "the connection timed out"
        );
        assert_eq!(
            HttpError::DnsError(DnsErrorPayload {
                rcode: Some("NXDOMAIN".to_owned()),
                info_code: None,
            })
            .to_string(),
            "the DNS lookup failed: NXDOMAIN"
        );
        assert_eq!(
            HttpError::Internal(Some("CORS".to_owned())).to_string(),
            "HTTP failed: CORS"
        );
    }
}
