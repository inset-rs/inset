//! HTTP in a browser: `fetch`, whose promise the app's executor awaits and whose response body
//! streams out of a `ReadableStream`. The browser's own fetch does the redirects, applies the
//! page's cookies and its CORS rules, and uses its proxy; a request dropped aborts the fetch.
//! A request's body is read whole before the fetch, so the browser holds it and resends it on a
//! 307 or a 308 by the fetch spec's own rules. A body made with trailers to send fails here with
//! [`HttpError::HttpRequestTrailerSectionSize`], as `fetch` sends none.
//!
//! A browser's `fetch` does not expose trailers, and hides a redirect response from the page: it
//! follows redirects up to its own limit, or with `redirect: "manual"` answers an opaque response
//! with no status or location. A policy that follows none fails with `TooManyRedirects` at the
//! first redirect, rather than answering it.
//!
//! ## Timeouts
//!
//! One `AbortController` is restarted through `setTimeout`: first for the head, which is the
//! closest a page comes to connect, then for the first body byte, then for the gap before each
//! next piece. A page sees no separate connection step, so `connect_timeout` bounds the fetch
//! until the head and `first_byte_timeout` the body after it; `between_bytes_timeout` restarts on
//! each piece. A timeout aborts the fetch with a reason naming the step, which becomes the error.
//!
//! ## What `fetch` tells as [`HttpError`]
//!
//! | `fetch` tells | [`HttpError`] |
//! | --- | --- |
//! | the connect timer aborted the fetch | `ConnectionTimeout` |
//! | the first-byte or between-bytes timer aborted it | `HttpResponseTimeout` |
//! | a `TimeoutError` from the browser | `HttpResponseTimeout` |
//! | an abort for another reason | `ConnectionTerminated` |
//! | a `TypeError`: a network failure, DNS, CORS, a refused connection | `Internal`, with the browser's message |
//! | an opaque redirect | `TooManyRedirects` |
//! | a URL `Request` will not take | `HttpRequestUriInvalid` |
//! | a status, header or stream the page cannot read | `HttpProtocolError` |

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use js_sys::{Reflect, Uint8Array};
use waft_embedder::{
    Body, BodyReadFuture, BodyStream, BodyTrailersFuture, HttpClient, HttpError, HttpFuture,
    RedirectPolicy, SendOptions,
};
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortController, Headers, ReadableStreamDefaultReader, Request as WebRequest, RequestInit,
    RequestRedirect, Response as WebResponse, ResponseType,
};

/// Why a timeout aborted the fetch: the step it bounded.
const CONNECT_TIMEOUT: &str = "waft-connect-timeout";
const FIRST_BYTE_TIMEOUT: &str = "waft-first-byte-timeout";
const BETWEEN_BYTES_TIMEOUT: &str = "waft-between-bytes-timeout";

/// HTTP over the page's `fetch`.
#[derive(Clone, Copy, Debug, Default)]
pub struct BrowserHttpClient;

impl BrowserHttpClient {
    pub const fn new() -> BrowserHttpClient {
        BrowserHttpClient
    }
}

impl HttpClient for BrowserHttpClient {
    fn send(&self, mut request: http::Request<Body>, options: SendOptions) -> HttpFuture {
        Box::pin(async move {
            let controller = AbortController::new()
                .map_err(|error| HttpError::Internal(Some(describe(&error))))?;
            // If this future is dropped before a response arrives, the fetch is aborted.
            let abort = AbortOnDrop(controller.clone());
            let window = web_sys::window()
                .ok_or_else(|| HttpError::Internal(Some("the page has no window".to_owned())))?;
            let deadlines = Deadlines::new(&window, &controller);
            let method = request.method().as_str().to_owned();
            let url = request.uri().to_string();
            let headers = request.headers().clone();
            let bytes = request.body_mut().read_all().await?;
            // `fetch` sends no request trailers, so a body that carries them fails here with the
            // one case that says a trailer section must be empty, rather than losing them.
            if request.body_mut().trailers().await?.is_some() {
                return Err(HttpError::HttpRequestTrailerSectionSize(Some(0)));
            }

            let init = RequestInit::new();
            init.set_method(&method);
            let web_headers =
                Headers::new().map_err(|error| HttpError::Internal(Some(describe(&error))))?;
            for (name, value) in &headers {
                let name = name.as_str();
                let value = String::from_utf8_lossy(value.as_bytes());
                web_headers
                    .append(name, &value)
                    .map_err(|_| HttpError::HttpProtocolError)?;
            }
            init.set_headers(&web_headers);
            init.set_signal(Some(&controller.signal()));
            init.set_redirect(redirect_mode(options.redirects));
            if !bytes.is_empty() {
                let data = Uint8Array::from(&bytes[..]);
                init.set_body(&JsValue::from(data));
            }
            let request = WebRequest::new_with_str_and_init(&url, &init)
                .map_err(|_| HttpError::HttpRequestUriInvalid)?;
            deadlines.start(head_timeout(&options), head_timeout_reason(&options));
            let response: WebResponse = JsFuture::from(window.fetch_with_request(&request))
                .await
                .map_err(|error| fetch_error(&error))?
                .dyn_into()
                .map_err(|error| HttpError::Internal(Some(describe(&error))))?;
            deadlines.start(options.first_byte_timeout, FIRST_BYTE_TIMEOUT);
            if response.type_() == ResponseType::Opaqueredirect {
                return Err(HttpError::TooManyRedirects);
            }

            let status = http::StatusCode::from_u16(response.status())
                .map_err(|_| HttpError::HttpProtocolError)?;
            let headers = response_headers(&response)?;
            let length = body_length(&headers);
            let stream = response.body();
            let ended = stream.is_none();
            if ended {
                deadlines.clear();
            }
            let reader = stream
                .map(|stream| {
                    stream
                        .get_reader()
                        .dyn_into::<ReadableStreamDefaultReader>()
                        .map_err(|_| HttpError::HttpProtocolError)
                })
                .transpose()?;
            let body = Body::from_stream(BrowserBodyStream {
                reader,
                pending: Vec::new(),
                ended,
                length,
                deadlines,
                between_bytes_timeout: options.between_bytes_timeout,
                _abort: abort,
            });
            let mut answer = http::Response::builder()
                .status(status)
                .version(http::Version::HTTP_11);
            for (name, value) in &headers {
                answer = answer.header(name, value);
            }
            answer.body(body).map_err(|_| HttpError::HttpProtocolError)
        })
    }
}

/// The timeout before the head: connect, or first byte when connect is unsaid.
fn head_timeout(options: &SendOptions) -> Option<Duration> {
    options.connect_timeout.or(options.first_byte_timeout)
}

/// Which step the head timeout is, for the error it becomes.
fn head_timeout_reason(options: &SendOptions) -> &'static str {
    if options.connect_timeout.is_some() {
        CONNECT_TIMEOUT
    } else {
        FIRST_BYTE_TIMEOUT
    }
}

/// `fetch`'s redirect mode for `policy`: the browser follows up to its own limit, or the first
/// redirect is the opaque response a policy that follows none fails on.
fn redirect_mode(policy: RedirectPolicy) -> RequestRedirect {
    match policy {
        RedirectPolicy::Follow { max } if max > 0 => RequestRedirect::Follow,
        RedirectPolicy::Follow { .. } | RedirectPolicy::Never => RequestRedirect::Manual,
    }
}

/// A fetch or a body read the browser failed: a step's timeout, a network failure, CORS or a bad
/// URL.
fn fetch_error(error: &JsValue) -> HttpError {
    let text = describe(error);
    if text.contains(CONNECT_TIMEOUT) {
        return HttpError::ConnectionTimeout;
    }
    if text.contains(FIRST_BYTE_TIMEOUT) || text.contains(BETWEEN_BYTES_TIMEOUT) {
        return HttpError::HttpResponseTimeout;
    }
    let name = Reflect::get(error, &JsValue::from_str("name"))
        .ok()
        .and_then(|name| name.as_string());
    match name.as_deref() {
        Some("TimeoutError") => HttpError::HttpResponseTimeout,
        Some("AbortError") => HttpError::ConnectionTerminated,
        // A TypeError is a network failure, DNS, CORS or a refused connection: the browser does
        // not say which, and WASI has no case for a page's CORS rule.
        _ => HttpError::Internal(Some(text)),
    }
}

fn describe(value: &JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            Reflect::get(value, &JsValue::from_str("message"))
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| format!("{value:?}"))
}

fn response_headers(response: &WebResponse) -> Result<http::HeaderMap, HttpError> {
    let headers = response.headers();
    let mut map = http::HeaderMap::new();
    let entries = headers.entries();
    loop {
        let next = entries.next().map_err(|_| HttpError::HttpProtocolError)?;
        if next.done() {
            break;
        }
        let pair: js_sys::Array = next
            .value()
            .dyn_into()
            .map_err(|_| HttpError::HttpProtocolError)?;
        let Some(name) = pair.get(0).as_string() else {
            continue;
        };
        let Some(value) = pair.get(1).as_string() else {
            continue;
        };
        let (Ok(name), Ok(value)) = (
            http::HeaderName::from_bytes(name.as_bytes()),
            http::HeaderValue::from_bytes(value.as_bytes()),
        ) else {
            continue;
        };
        map.append(name, value);
    }
    Ok(map)
}

/// The body's length when the headers say it and the body is not compressed, as `fetch` hands
/// the app decompressed bytes.
fn body_length(headers: &http::HeaderMap) -> Option<u64> {
    let compressed = headers
        .get(http::header::CONTENT_ENCODING)
        .is_some_and(|encoding| !encoding.as_bytes().eq_ignore_ascii_case(b"identity"));
    if compressed {
        return None;
    }
    headers
        .get(http::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
}

/// One timer on the page, restarted by each step of the exchange: when it fires, it aborts the
/// fetch with a reason naming the step, which [`fetch_error`] turns into the step's case.
struct Deadlines {
    window: web_sys::Window,
    on_timeout: Closure<dyn FnMut()>,
    handle: Cell<Option<i32>>,
    reason: Rc<Cell<&'static str>>,
}

impl Deadlines {
    fn new(window: &web_sys::Window, controller: &AbortController) -> Deadlines {
        let reason = Rc::new(Cell::new(CONNECT_TIMEOUT));
        let on_timeout = {
            let controller = controller.clone();
            let reason = Rc::clone(&reason);
            Closure::wrap(Box::new(move || {
                controller.abort_with_reason(&JsValue::from_str(reason.get()));
            }) as Box<dyn FnMut()>)
        };
        Deadlines {
            window: window.clone(),
            on_timeout,
            handle: Cell::new(None),
            reason,
        }
    }

    /// Starts `timeout` for `reason`, replacing the timer already running.
    fn start(&self, timeout: Option<Duration>, reason: &'static str) {
        self.clear();
        let Some(timeout) = timeout else {
            return;
        };
        self.reason.set(reason);
        let millis = timeout.as_millis().min(i32::MAX as u128) as i32;
        let function: &js_sys::Function = self.on_timeout.as_ref().unchecked_ref();
        if let Ok(handle) = self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(function, millis)
        {
            self.handle.set(Some(handle));
        }
    }

    fn clear(&self) {
        if let Some(handle) = self.handle.take() {
            self.window.clear_timeout_with_handle(handle);
        }
    }
}

impl Drop for Deadlines {
    fn drop(&mut self) {
        self.clear();
    }
}

/// Aborts the fetch when the response or the future holding it is dropped.
struct AbortOnDrop(AbortController);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The response body, read from the browser's stream a piece at a time.
struct BrowserBodyStream {
    reader: Option<ReadableStreamDefaultReader>,
    pending: Vec<u8>,
    ended: bool,
    /// The body's length, which the response head names rather than the stream.
    length: Option<u64>,
    /// The one timer, restarted for each piece of the body.
    deadlines: Deadlines,
    between_bytes_timeout: Option<Duration>,
    _abort: AbortOnDrop,
}

impl BodyStream for BrowserBodyStream {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        Box::pin(async move {
            loop {
                if !self.pending.is_empty() {
                    let read = self.pending.len().min(buf.len());
                    buf[..read].copy_from_slice(&self.pending[..read]);
                    self.pending.drain(..read);
                    return Ok(read);
                }
                if self.ended {
                    return Ok(0);
                }
                let Some(reader) = &self.reader else {
                    self.ended = true;
                    return Ok(0);
                };
                let value = JsFuture::from(reader.read())
                    .await
                    .map_err(|error| fetch_error(&error))?;
                self.deadlines
                    .start(self.between_bytes_timeout, BETWEEN_BYTES_TIMEOUT);
                let done = Reflect::get(&value, &JsValue::from_str("done"))
                    .ok()
                    .and_then(|done| done.as_bool())
                    .unwrap_or(true);
                if done {
                    self.ended = true;
                    self.deadlines.clear();
                    return Ok(0);
                }
                let chunk = Reflect::get(&value, &JsValue::from_str("value"))
                    .map_err(|_| HttpError::HttpProtocolError)?;
                self.pending = Uint8Array::new(&chunk).to_vec();
            }
        })
    }

    fn trailers(&mut self) -> BodyTrailersFuture<'_> {
        // `fetch` gives no trailers.
        Box::pin(std::future::ready(Ok(None)))
    }

    fn length(&self) -> Option<u64> {
        self.length
    }
}
