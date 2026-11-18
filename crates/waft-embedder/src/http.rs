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

use bytes::Bytes;
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
    ///
    /// A redirect that resends the request's body — a 307 or a 308 — is followed only by a client
    /// that can send the body again: one that holds it, or [`Body::try_clone`] answered `Some`
    /// for it. A client that streams the body, as a WebAssembly component's does, answers the
    /// redirect response instead, so the app can decide what to do with a body that is gone.
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
    ///
    /// A 307 or a 308 resends the request's method and body: a client follows it when it holds
    /// the body, or [`Body::try_clone`] answers `Some` for it; otherwise it answers the redirect
    /// response as it is, rather than sending a request whose body is missing, and leaves the
    /// decision to the app.
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
    ///
    /// Called only after [`read`](Self::read) has answered `0`; asking again answers the same
    /// fields, so an implementation over a future that may be awaited once keeps its answer. On
    /// a request body this answers the fields to send.
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
///
/// The fields that come after a body — its trailers — are read with [`trailers`](Self::trailers)
/// once the body has ended: it reads what is left of the body first, discarding those bytes to
/// reach its end, as HTTP sends its trailers only after the last byte, so asking for them never
/// waits for a reader that never comes. A request carries its own with
/// [`with_trailers`](Self::with_trailers), which the body answers once its bytes have been read,
/// so the fields may be made from the bytes streamed (a checksum). Asking again answers the same
/// fields. A body made from bytes can be sent again with [`try_clone`](Self::try_clone), which is
/// what a redirect that resends its body needs.
pub struct Body(BodyKind);

/// What a body holds.
enum BodyKind {
    /// No bytes at all.
    Empty,
    /// Bytes in memory, and how many of them have been read.
    Bytes { bytes: Bytes, read: usize },
    /// A body with the fields to send after it, which it answers once its bytes are read. Its
    /// own stream's fields, if it had any, are replaced.
    Trailered {
        inner: Box<BodyKind>,
        trailers: HeaderMap,
    },
    /// A stream, read as the platform delivers it.
    Stream(Box<dyn BodyStream>),
}

impl BodyStream for BodyKind {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        match self {
            BodyKind::Empty => Box::pin(std::future::ready(Ok(0))),
            BodyKind::Bytes { bytes, read } => {
                let start = *read;
                let end = (start + buf.len()).min(bytes.len());
                buf[..end - start].copy_from_slice(&bytes[start..end]);
                *read = end;
                Box::pin(std::future::ready(Ok(end - start)))
            }
            BodyKind::Trailered { inner, .. } => inner.read(buf),
            BodyKind::Stream(stream) => stream.read(buf),
        }
    }

    fn trailers(&mut self) -> BodyTrailersFuture<'_> {
        match self {
            BodyKind::Empty | BodyKind::Bytes { .. } => Box::pin(std::future::ready(Ok(None))),
            BodyKind::Trailered { trailers, .. } => {
                let trailers = trailers.clone();
                Box::pin(std::future::ready(Ok(Some(trailers))))
            }
            BodyKind::Stream(stream) => stream.trailers(),
        }
    }

    fn length(&self) -> Option<u64> {
        match self {
            BodyKind::Empty => Some(0),
            BodyKind::Bytes { bytes, .. } => Some(bytes.len() as u64),
            BodyKind::Trailered { inner, .. } => inner.length(),
            BodyKind::Stream(stream) => stream.length(),
        }
    }
}

/// How much one whole read asks for at a time.
const READ_CHUNK: usize = 64 * 1024;

impl Body {
    /// No bytes at all: a `GET`'s body, or a response with nothing in it.
    pub fn empty() -> Body {
        Body(BodyKind::Empty)
    }

    /// The bytes `stream` reads, as they come.
    pub fn from_stream(stream: impl BodyStream + 'static) -> Body {
        Body(BodyKind::Stream(Box::new(stream)))
    }

    /// `trailers`, sent after this body ends: the body answers them once its bytes have been
    /// read, as HTTP sends its trailers after the last byte. A body whose stream has fields of
    /// its own has them replaced. A client whose protocol layer cannot send trailers fails the
    /// request rather than dropping them. Over HTTP/1.1 the request names its trailers in a
    /// `Trailer` header, as RFC 9110 §6.6.2 asks of a sender: a client may drop a field it was
    /// not told of (hyper does), and a body's trailers may not be known until it ends, so the
    /// request says them, not the client.
    pub fn with_trailers(self, trailers: HeaderMap) -> Body {
        Body(BodyKind::Trailered {
            inner: Box::new(self.0),
            trailers,
        })
    }

    /// A second body with the same bytes, for a request a client sends again on a redirect that
    /// resends its body. `Some` for a body made from bytes and for an empty one, with the fields
    /// it sends; `None` for one that streams, whose bytes are gone once they are read and whose
    /// fields may depend on them.
    pub fn try_clone(&self) -> Option<Body> {
        Some(Body(self.0.clone_for_resend()?))
    }

    /// Reads the next bytes of the body into `buf`, answering how many it read, `0` at the end:
    /// a piece at a time, as the platform delivers it.
    pub fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        self.0.read(buf)
    }

    /// The whole body, read through: the bytes a request carries, or a response's.
    pub fn read_all(&mut self) -> BodyReadAllFuture<'_> {
        match &mut self.0 {
            BodyKind::Empty => Box::pin(std::future::ready(Ok(Vec::new()))),
            BodyKind::Bytes { bytes, read } => {
                let start = *read;
                *read = bytes.len();
                Box::pin(std::future::ready(Ok(bytes.slice(start..).to_vec())))
            }
            body => Box::pin(read_all(body)),
        }
    }

    /// The fields that came after the body, once it has ended: reads what is left of the body
    /// first, discarding those bytes to reach its end, as HTTP sends its trailers only after its
    /// last byte, so asking for them never waits for a reader that never comes. The bytes are
    /// discarded into a buffer on the stack, so a body that has already ended costs no allocation.
    /// Asking again answers the same fields, which a client that sends them keeps; `None` where
    /// the response has none, and always `None` on a client whose protocol layer does not expose
    /// them.
    pub fn trailers(&mut self) -> BodyTrailersFuture<'_> {
        Box::pin(async move {
            skip_to_end(&mut self.0).await?;
            self.0.trailers().await
        })
    }

    /// How many bytes the whole body carries, when its sender said so: `Some(0)` for an empty
    /// body, and `None` for one whose length is unknown.
    pub fn length(&self) -> Option<u64> {
        self.0.length()
    }
}

impl From<Vec<u8>> for Body {
    fn from(bytes: Vec<u8>) -> Body {
        Body(BodyKind::Bytes {
            bytes: Bytes::from(bytes),
            read: 0,
        })
    }
}

impl From<&'static [u8]> for Body {
    fn from(bytes: &'static [u8]) -> Body {
        Body(BodyKind::Bytes {
            bytes: Bytes::from_static(bytes),
            read: 0,
        })
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
            BodyKind::Empty => formatter.write_str("Body(empty)"),
            kind => formatter
                .debug_struct("Body")
                .field("length", &kind.length())
                .finish(),
        }
    }
}

impl BodyKind {
    /// A body over the same bytes, read from its start, when the bytes are held rather than
    /// streamed: what a client sends again on a redirect that resends its body.
    fn clone_for_resend(&self) -> Option<BodyKind> {
        match self {
            BodyKind::Empty => Some(BodyKind::Empty),
            BodyKind::Bytes { bytes, .. } => Some(BodyKind::Bytes {
                bytes: bytes.clone(),
                read: 0,
            }),
            BodyKind::Trailered { inner, trailers } => Some(BodyKind::Trailered {
                inner: Box::new(inner.clone_for_resend()?),
                trailers: trailers.clone(),
            }),
            BodyKind::Stream(_) => None,
        }
    }
}

async fn read_all(stream: &mut dyn BodyStream) -> Result<Vec<u8>, HttpError> {
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

/// Reads `stream` to its end, discarding what is left: what asking for a body's trailers does
/// before it can answer them.
async fn skip_to_end(stream: &mut dyn BodyStream) -> Result<(), HttpError> {
    let mut discarded = [0u8; 8192];
    while stream.read(&mut discarded).await? != 0 {}
    Ok(())
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
        assert_eq!(body.length(), Some(11));
        assert_eq!(now(body.read_all()).unwrap(), b"hello, body");
        assert_eq!(now(body.trailers()).unwrap(), None, "no trailers came");
    }

    #[test]
    fn an_empty_body_carries_no_bytes_where_a_stream_of_unknown_length_carries_some_unknown() {
        assert_eq!(
            Body::empty().length(),
            Some(0),
            "an empty body is zero bytes"
        );
        struct Unknown;
        impl BodyStream for Unknown {
            fn read<'a>(&'a mut self, _buf: &'a mut [u8]) -> BodyReadFuture<'a> {
                Box::pin(std::future::ready(Ok(0)))
            }
            fn trailers(&mut self) -> BodyTrailersFuture<'_> {
                Box::pin(std::future::ready(Ok(None)))
            }
            fn length(&self) -> Option<u64> {
                None
            }
        }
        assert_eq!(Body::from_stream(Unknown).length(), None);
    }

    /// The stream every test below reads: its bytes in pieces, and trailers after them.
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

    #[test]
    fn a_streamed_body_is_read_a_piece_at_a_time() {
        let mut body = Body::from_stream(Pieces(b"streamed body".to_vec(), 0));
        assert_eq!(now(body.read_all()).unwrap(), b"streamed body");
        assert_eq!(
            now(body.trailers()).unwrap().unwrap()["x-checksum"],
            "7",
            "the trailers came after the body"
        );
    }

    #[test]
    fn asking_for_the_trailers_again_answers_the_same() {
        let mut body = Body::from_stream(Pieces(b"streamed body".to_vec(), 0));
        let first = now(body.trailers()).unwrap().expect("trailers came");
        let second = now(body.trailers()).unwrap().expect("trailers came again");
        assert_eq!(first, second, "a second asking answers the same fields");
    }

    #[test]
    fn asking_for_the_trailers_reads_the_rest_of_the_body_first() {
        let mut body = Body::from_stream(Pieces(b"streamed body".to_vec(), 0));
        let trailers = now(body.trailers())
            .expect("the body ended")
            .expect("trailers came");
        assert_eq!(trailers["x-checksum"], "7");
        let mut chunk = [0u8; 8];
        assert_eq!(
            now(body.read(&mut chunk)).unwrap(),
            0,
            "the body was drained to its end before the trailers answered"
        );
    }

    #[test]
    fn a_body_with_trailers_to_send_answers_them_once_its_bytes_are_read() {
        let mut trailers = HeaderMap::new();
        trailers.insert("x-checksum", 7u32.into());
        let mut body = Body::from(b"a body".to_vec()).with_trailers(trailers.clone());
        assert_eq!(now(body.read_all()).unwrap(), b"a body");
        assert_eq!(now(body.trailers()).unwrap(), Some(trailers.clone()));
        assert_eq!(
            now(body.trailers()).unwrap(),
            Some(trailers),
            "asking again answers the same fields"
        );
    }

    #[test]
    fn a_streams_own_trailers_may_be_made_from_its_bytes() {
        /// A body that counts its bytes and answers the count as a trailer, as a checksum is
        /// made from what was read.
        struct Counted {
            bytes: Vec<u8>,
            read: usize,
        }
        impl BodyStream for Counted {
            fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
                let read = (self.bytes.len() - self.read).min(buf.len());
                buf[..read].copy_from_slice(&self.bytes[self.read..self.read + read]);
                self.read += read;
                Box::pin(std::future::ready(Ok(read)))
            }
            fn trailers(&mut self) -> BodyTrailersFuture<'_> {
                let mut trailers = HeaderMap::new();
                trailers.insert("x-length", self.read.into());
                Box::pin(std::future::ready(Ok(Some(trailers))))
            }
            fn length(&self) -> Option<u64> {
                Some(self.bytes.len() as u64)
            }
        }
        let mut body = Body::from_stream(Counted {
            bytes: b"count these".to_vec(),
            read: 0,
        });
        let trailers = now(body.trailers()).unwrap().unwrap();
        assert_eq!(
            trailers["x-length"], "11",
            "the trailers answered after the bytes were read, so they can be made from them"
        );
    }

    #[test]
    fn a_body_made_from_bytes_is_sent_again_and_a_stream_is_not() {
        let body = Body::from(b"resend me".to_vec());
        let mut again = body.try_clone().expect("bytes can be sent again");
        assert_eq!(again.length(), body.length());
        assert_eq!(now(again.read_all()).unwrap(), b"resend me");
        assert_eq!(
            now(again.trailers()).unwrap(),
            None,
            "a body without trailers clones without them"
        );

        let streamed = Body::from_stream(Pieces(b"streamed".to_vec(), 0));
        assert!(
            streamed.try_clone().is_none(),
            "a stream's bytes are gone once they are read"
        );

        let mut trailers = HeaderMap::new();
        trailers.insert("x-checksum", 7u32.into());
        let wrapped = Body::from(b"a body".to_vec()).with_trailers(trailers.clone());
        assert!(
            wrapped.try_clone().is_some(),
            "bytes and the fixed trailers they send go again together"
        );
    }

    #[test]
    fn a_body_with_fixed_trailers_is_sent_again_with_them() {
        let mut trailers = HeaderMap::new();
        trailers.insert("x-checksum", 7u32.into());
        let body = Body::from(b"a body".to_vec()).with_trailers(trailers.clone());
        let mut again = body
            .try_clone()
            .expect("a body of bytes and fixed trailers is sent again");
        assert_eq!(now(again.read_all()).unwrap(), b"a body");
        assert_eq!(now(again.trailers()).unwrap(), Some(trailers.clone()));
        let mut third = body.try_clone().unwrap();
        assert_eq!(
            now(third.trailers()).unwrap(),
            Some(trailers),
            "each clone answers the same fields"
        );
    }

    #[test]
    fn trailers_given_to_a_stream_replace_its_own() {
        let mut trailers = HeaderMap::new();
        trailers.insert("x-other", 1u32.into());
        let mut body =
            Body::from_stream(Pieces(b"streamed body".to_vec(), 0)).with_trailers(trailers.clone());
        assert_eq!(
            now(body.trailers()).unwrap(),
            Some(trailers),
            "the given fields are the ones sent"
        );
        assert!(
            body.try_clone().is_none(),
            "a stream's bytes are gone once they are read"
        );
    }

    #[test]
    fn an_empty_body_is_sent_again_with_nothing() {
        let empty = Body::empty();
        let mut again = empty.try_clone().expect("nothing to lose");
        assert_eq!(
            again.length(),
            Some(0),
            "an empty body carries no bytes, where a stream of unknown length says nothing"
        );
        assert_eq!(now(again.read_all()).unwrap(), Vec::<u8>::new());
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
