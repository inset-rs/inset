//! HTTP in a browser: `fetch`, whose promise the app's executor awaits and whose response body
//! streams out of a `ReadableStream`. The browser's own fetch does the redirects, applies the
//! page's cookies and its CORS rules, and uses its proxy; a request dropped aborts the fetch, and
//! a `SendOptions` timeout aborts it through `AbortSignal.timeout`.
//!
//! A browser's `fetch` does not expose trailers, and hides a redirect response from the page: it
//! follows redirects up to its own limit, or with `redirect: "manual"` answers an opaque response
//! with no status or location. A policy that follows none fails with `TooManyRedirects` at the
//! first redirect, rather than answering it.

use js_sys::{Reflect, Uint8Array};
use waft_embedder::{
    Body, BodyReadFuture, BodyStream, BodyTrailersFuture, HttpClient, HttpError, HttpFuture,
    RedirectPolicy, SendOptions,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    AbortController, AbortSignal, Headers, ReadableStreamDefaultReader, Request as WebRequest,
    RequestInit, RequestRedirect, Response as WebResponse, ResponseType,
};

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
            let controller =
                AbortController::new().map_err(|error| HttpError::Protocol(describe(&error)))?;
            // If this future is dropped before a response arrives, the fetch is aborted.
            let abort = AbortOnDrop(controller.clone());
            let method = request.method().as_str().to_owned();
            let url = request.uri().to_string();
            let headers = request.headers().clone();
            let bytes = request.body_mut().read_all().await?;

            let init = RequestInit::new();
            init.set_method(&method);
            let web_headers =
                Headers::new().map_err(|error| HttpError::Protocol(describe(&error)))?;
            for (name, value) in &headers {
                let name = name.as_str();
                let value = String::from_utf8_lossy(value.as_bytes());
                web_headers
                    .append(name, &value)
                    .map_err(|error| HttpError::Protocol(describe(&error)))?;
            }
            init.set_headers(&web_headers);
            init.set_signal(Some(&signal(&controller, &options)));
            init.set_redirect(redirect_mode(options.redirects));
            if !bytes.is_empty() {
                let data = Uint8Array::from(&bytes[..]);
                init.set_body(&JsValue::from(data));
            }
            let request = WebRequest::new_with_str_and_init(&url, &init)
                .map_err(|error| HttpError::Protocol(describe(&error)))?;
            let window = web_sys::window()
                .ok_or_else(|| HttpError::Protocol("the page has no window".to_owned()))?;
            let response: WebResponse = JsFuture::from(window.fetch_with_request(&request))
                .await
                .map_err(|error| fetch_error(&error))?
                .dyn_into()
                .map_err(|error| HttpError::Protocol(describe(&error)))?;
            if response.type_() == ResponseType::Opaqueredirect {
                return Err(HttpError::TooManyRedirects);
            }

            let status = http::StatusCode::from_u16(response.status())
                .map_err(|_| HttpError::Protocol(format!("status {}", response.status())))?;
            let headers = response_headers(&response)?;
            let length = body_length(&headers);
            let stream = response.body();
            let ended = stream.is_none();
            let reader = stream
                .map(|stream| {
                    stream
                        .get_reader()
                        .dyn_into::<ReadableStreamDefaultReader>()
                        .map_err(|error| HttpError::Protocol(describe(&error)))
                })
                .transpose()?;
            let body = Body::from_stream(BrowserBodyStream {
                reader,
                pending: Vec::new(),
                ended,
                length,
                _abort: abort,
            });
            let mut answer = http::Response::builder()
                .status(status)
                .version(http::Version::HTTP_11);
            for (name, value) in &headers {
                answer = answer.header(name, value);
            }
            answer
                .body(body)
                .map_err(|error| HttpError::Protocol(error.to_string()))
        })
    }
}

/// What aborts the fetch: dropping the request, and its timeout when it has one.
fn signal(controller: &AbortController, options: &SendOptions) -> AbortSignal {
    let dropped = controller.signal();
    match options.timeout {
        Some(timeout) => {
            let timed_out = AbortSignal::timeout_with_f64(timeout.as_secs_f64() * 1000.0);
            AbortSignal::any(&js_sys::Array::of2(&dropped, &timed_out))
        }
        None => dropped,
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

/// A fetch or a body read the browser failed: its timeout's `TimeoutError`, or a network
/// failure, CORS or a bad URL.
fn fetch_error(error: &JsValue) -> HttpError {
    let name = Reflect::get(error, &JsValue::from_str("name"))
        .ok()
        .and_then(|name| name.as_string());
    if name.as_deref() == Some("TimeoutError") {
        HttpError::Timeout
    } else {
        HttpError::Connection(describe(error))
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
        let next = entries
            .next()
            .map_err(|error| HttpError::Protocol(describe(&error)))?;
        if next.done() {
            break;
        }
        let pair: js_sys::Array = next
            .value()
            .dyn_into()
            .map_err(|error| HttpError::Protocol(describe(&error)))?;
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
                let done = Reflect::get(&value, &JsValue::from_str("done"))
                    .ok()
                    .and_then(|done| done.as_bool())
                    .unwrap_or(true);
                if done {
                    self.ended = true;
                    return Ok(0);
                }
                let chunk = Reflect::get(&value, &JsValue::from_str("value"))
                    .map_err(|error| HttpError::Protocol(describe(&error)))?;
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
