//! HTTP on macOS and iOS: `NSURLSession`'s data tasks, one session for the app, whose delegate
//! hands each response head and body piece to the future awaiting it. The session is the
//! system's, so it uses the system proxy, the system's certificates and the system's connection
//! reuse without any of that written here.
//!
//! A response's head arrives as soon as the session has it, and its body streams as the data
//! arrives. Dropping the response or the future cancels its task. Request bodies are read whole
//! before the request is made: `NSURLSession` takes a body as data or as an `NSInputStream`, and
//! this does not wrap a Waft future in one. Redirects are counted here so that a policy's limit
//! is the one that applies, and TLS challenges are left to the session's default handling.
//!
//! `NSHTTPURLResponse` joins a repeated response header into one value, and says nothing of the
//! protocol version, so a response answers each header once and as HTTP/1.1 whatever was spoken.
//!
//! ## Timeouts
//!
//! `between_bytes_timeout` is the request's `timeoutInterval`, which `NSURLSession` applies as
//! the gap between pieces of data; `connect_timeout` and `first_byte_timeout` are timers this
//! client starts itself from the request's start, the first until the response head arrives and
//! the second until the first body byte does. `None` leaves the request interval at a day and the
//! timers unstarted.
//!
//! ## Back-pressure
//!
//! A response's data is held in an unbounded queue as the session delivers it, up to
//! [`BUFFER_LIMIT`] bytes: at that point the task is suspended, which stops the session reading
//! from the network, and the reader resumes it once the queue falls below the limit. A slow
//! reader therefore holds a bounded amount of memory while the connection stays open, and no
//! received byte is dropped, which an HTTP body cannot be.
//!
//! ## `NSURLSession` failures as [`HttpError`]
//!
//! | `NSURLError` | [`HttpError`] |
//! | --- | --- |
//! | `TimedOut` (the request interval, between bytes) | `HttpResponseTimeout` |
//! | `BadURL`, `UnsupportedURL` | `HttpRequestUriInvalid` |
//! | `CannotFindHost`, `DNSLookupFailed` | `DnsError` |
//! | `CannotConnectToHost` | `ConnectionRefused` |
//! | `NetworkConnectionLost` | `ConnectionTerminated` |
//! | `Cancelled` | `ConnectionTerminated` |
//! | `NotConnectedToInternet`, `DataNotAllowed`, `InternationalRoamingOff`, `CallIsActive`, `ResourceUnavailable`, `CannotLoadFromNetwork` | `DestinationUnavailable` |
//! | `HTTPTooManyRedirects` | `TooManyRedirects` |
//! | `RedirectToNonExistentLocation` | `DestinationNotFound` |
//! | `SecureConnectionFailed` | `TlsProtocolError` |
//! | `ServerCertificateHasBadDate`, `ServerCertificateUntrusted`, `ServerCertificateHasUnknownRoot`, `ServerCertificateNotYetValid`, `ClientCertificateRejected`, `ClientCertificateRequired`, `AppTransportSecurityRequiresSecureConnection` | `TlsCertificateError` |
//! | `UserCancelledAuthentication`, `UserAuthenticationRequired` | `HttpRequestDenied` |
//! | `BadServerResponse`, `CannotParseResponse`, `ZeroByteResource`, `CannotDecodeRawData`, `CannotDecodeContentData` | `HttpProtocolError` |
//! | anything else | `Internal`, with the system's description |
#![allow(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use block2::DynBlock;
use dispatch2::{DispatchQueue, DispatchQueueGlobalPriority, DispatchTime, GlobalQueueIdentifier};
use http::{HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode, Uri, Version};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{
    NSData, NSError, NSHTTPURLResponse, NSMutableURLRequest, NSOperationQueue, NSString, NSURL,
    NSURLErrorAppTransportSecurityRequiresSecureConnection, NSURLErrorBadServerResponse,
    NSURLErrorBadURL, NSURLErrorCallIsActive, NSURLErrorCancelled, NSURLErrorCannotConnectToHost,
    NSURLErrorCannotDecodeContentData, NSURLErrorCannotDecodeRawData, NSURLErrorCannotFindHost,
    NSURLErrorCannotLoadFromNetwork, NSURLErrorCannotParseResponse,
    NSURLErrorClientCertificateRejected, NSURLErrorClientCertificateRequired,
    NSURLErrorDNSLookupFailed, NSURLErrorDataNotAllowed, NSURLErrorHTTPTooManyRedirects,
    NSURLErrorInternationalRoamingOff, NSURLErrorNetworkConnectionLost,
    NSURLErrorNotConnectedToInternet, NSURLErrorRedirectToNonExistentLocation,
    NSURLErrorResourceUnavailable, NSURLErrorSecureConnectionFailed,
    NSURLErrorServerCertificateHasBadDate, NSURLErrorServerCertificateHasUnknownRoot,
    NSURLErrorServerCertificateNotYetValid, NSURLErrorServerCertificateUntrusted,
    NSURLErrorTimedOut, NSURLErrorUnsupportedURL, NSURLErrorUserAuthenticationRequired,
    NSURLErrorUserCancelledAuthentication, NSURLErrorZeroByteResource, NSURLRequest, NSURLResponse,
    NSURLSession, NSURLSessionDataDelegate, NSURLSessionDataTask, NSURLSessionDelegate,
    NSURLSessionResponseDisposition, NSURLSessionTask, NSURLSessionTaskDelegate,
};
use waft_embedder::{
    Body, BodyReadFuture, BodyStream, BodyTrailersFuture, DnsErrorPayload, HttpClient, HttpError,
    HttpFuture, RedirectPolicy, SendOptions,
};

use crate::abort_on_panic;

/// Waft's HTTP client on Apple's systems: one `NSURLSession` for the app, whose delegate routes
/// each task's events to the exchange waiting for them.
pub struct AppleHttpClient {
    session: Retained<NSURLSession>,
    exchanges: Arc<Exchanges>,
    /// The session holds its delegate until it is invalidated, which dropping the client does.
    _delegate: Retained<HttpDelegate>,
}

impl Drop for AppleHttpClient {
    fn drop(&mut self) {
        // Without this the session and its delegate outlive the client: a session keeps its
        // delegate until it is invalidated.
        self.session.invalidateAndCancel();
    }
}

impl AppleHttpClient {
    /// A client over the system's default session: the system proxy, the system's certificates,
    /// and the system's disk cache for the responses that allow it.
    pub fn new() -> AppleHttpClient {
        let exchanges = Arc::new(Exchanges::default());
        let delegate = HttpDelegate::new(Arc::clone(&exchanges));
        let configuration =
            objc2_foundation::NSURLSessionConfiguration::defaultSessionConfiguration();
        // A queue of the session's own, not the main queue: a delegate callback may land while
        // the main thread is inside the app, and this way it only wakes the future.
        let queue = NSOperationQueue::new();
        queue.setMaxConcurrentOperationCount(1);
        let delegate_ref: &HttpDelegate = &delegate;
        // SAFETY: the delegate implements `NSURLSessionDelegate` and outlives the session; all its
        // callbacks come on the queue set here, whose threads only wake the app's futures.
        let session = unsafe {
            NSURLSession::sessionWithConfiguration_delegate_delegateQueue(
                &configuration,
                Some(ProtocolObject::from_ref(delegate_ref)),
                Some(&queue),
            )
        };
        AppleHttpClient {
            session,
            exchanges,
            _delegate: delegate,
        }
    }
}

impl Default for AppleHttpClient {
    fn default() -> AppleHttpClient {
        AppleHttpClient::new()
    }
}

impl HttpClient for AppleHttpClient {
    fn send(&self, request: Request<Body>, options: SendOptions) -> HttpFuture {
        let exchange = Arc::new(Exchange::new(&options));
        let loaded = load(
            self.session.clone(),
            Arc::clone(&self.exchanges),
            request,
            Arc::clone(&exchange),
            options,
        );
        Box::pin(async move {
            let transfer = loaded.await?;
            let head = HeadFuture {
                exchange: Arc::clone(&exchange),
            }
            .await?;
            let mut answer = Response::builder()
                .status(head.status)
                .version(head.version);
            for (name, value) in &head.headers {
                answer = answer.header(name, value);
            }
            let body = Body::from_stream(HttpBodyStream {
                exchange: Arc::clone(&exchange),
                length: head.length,
                _transfer: transfer,
            });
            answer.body(body).map_err(|_| HttpError::HttpProtocolError)
        })
    }
}

/// Builds the request and starts its task, answering what keeps it alive and cancels it when
/// dropped.
async fn load(
    session: Retained<NSURLSession>,
    exchanges: Arc<Exchanges>,
    mut request: Request<Body>,
    exchange: Arc<Exchange>,
    options: SendOptions,
) -> Result<Transfer, HttpError> {
    let url = url_of(request.uri())?;
    let method = request.method().as_str().to_owned();
    let headers = request.headers().clone();
    let bytes = request.body_mut().read_all().await?;
    let request = NSMutableURLRequest::new();
    request.setURL(Some(&url));
    request.setHTTPMethod(&NSString::from_str(&method));
    // `NSURLSession` applies its request interval as the gap between pieces of data, which is
    // WASI's between-bytes timeout. Without one, a day is far past any exchange.
    request.setTimeoutInterval(
        options
            .between_bytes_timeout
            .map(|timeout| timeout.as_secs_f64())
            .unwrap_or(24.0 * 60.0 * 60.0),
    );
    // `addValue` joins a repeated header's values with commas, as HTTP allows for every header
    // but `Set-Cookie`, which a request does not send.
    for (name, value) in &headers {
        request.addValue_forHTTPHeaderField(
            &NSString::from_str(&String::from_utf8_lossy(value.as_bytes())),
            &NSString::from_str(name.as_str()),
        );
    }
    if !bytes.is_empty() {
        request.setHTTPBody(Some(&NSData::from_vec(bytes)));
    }
    let task = session.dataTaskWithRequest(&request);
    exchanges
        .lock()
        .insert(task.taskIdentifier(), Arc::clone(&exchange));
    exchange.set_task(&task);
    if let Some(timeout) = options.connect_timeout {
        start_timer(
            Arc::clone(&exchange),
            task.clone(),
            timeout,
            Timeout::Connect,
        );
    }
    if let Some(timeout) = options.first_byte_timeout {
        start_timer(
            Arc::clone(&exchange),
            task.clone(),
            timeout,
            Timeout::FirstByte,
        );
    }
    task.resume();
    Ok(Transfer { task, exchanges })
}

fn url_of(uri: &Uri) -> Result<Retained<NSURL>, HttpError> {
    let scheme = uri.scheme_str();
    if !matches!(scheme, Some("http") | Some("https")) || uri.host().is_none() {
        return Err(HttpError::HttpRequestUriInvalid);
    }
    NSURL::URLWithString(&NSString::from_str(&uri.to_string()))
        .ok_or(HttpError::HttpRequestUriInvalid)
}

/// What keeps one request alive: its task, which is cancelled when this is dropped, and the
/// exchange the delegate answers into, which is forgotten on the way.
struct Transfer {
    task: Retained<NSURLSessionDataTask>,
    exchanges: Arc<Exchanges>,
}

impl Drop for Transfer {
    fn drop(&mut self) {
        self.task.cancel();
        self.exchanges.lock().remove(&self.task.taskIdentifier());
    }
}

/// The exchanges of the tasks in flight, by task number, which the delegate finds each event's
/// owner through.
#[derive(Default)]
struct Exchanges {
    by_task: Mutex<BTreeMap<usize, Arc<Exchange>>>,
}

impl Exchanges {
    fn lock(&self) -> MutexGuard<'_, BTreeMap<usize, Arc<Exchange>>> {
        self.by_task.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn get(&self, task: &NSURLSessionTask) -> Option<Arc<Exchange>> {
        self.lock().get(&task.taskIdentifier()).cloned()
    }
}

/// The response head, as the session gave it.
struct Head {
    status: StatusCode,
    version: Version,
    headers: HeaderMap,
    /// The body's length when the headers say it and the body is not compressed.
    length: Option<u64>,
}

/// What has arrived of one response, and who waits for the rest.
struct Exchange {
    state: Mutex<State>,
}

struct State {
    head: Option<Result<Head, HttpError>>,
    head_waiter: Option<Waker>,
    chunks: VecDeque<Vec<u8>>,
    /// How many bytes the queue holds, which back-pressure bounds.
    buffered: usize,
    /// Whether the session's task is suspended for want of a reader.
    paused: bool,
    /// The task, for suspending and resuming it, and for the timers.
    task: Option<Retained<NSURLSessionDataTask>>,
    body_waiter: Option<Waker>,
    /// Whether a body byte has arrived since the head, which the first-byte timer watches.
    first_body_byte: bool,
    ended: bool,
    error: Option<HttpError>,
    redirects_left: usize,
    redirect_policy: RedirectPolicy,
}

/// How much a response's delivered but unread data may hold before its task is suspended: 256
/// KiB, plus at most the piece the session was already delivering.
const BUFFER_LIMIT: usize = 256 * 1024;

impl Exchange {
    fn new(options: &SendOptions) -> Exchange {
        let redirects_left = match options.redirects {
            RedirectPolicy::Follow { max } => max,
            RedirectPolicy::Never => 0,
        };
        Exchange {
            state: Mutex::new(State {
                head: None,
                head_waiter: None,
                chunks: VecDeque::new(),
                buffered: 0,
                paused: false,
                task: None,
                body_waiter: None,
                first_body_byte: false,
                ended: false,
                error: None,
                redirects_left,
                redirect_policy: options.redirects,
            }),
        }
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn responded(&self, head: Result<Head, HttpError>) {
        let waiter = {
            let mut state = self.lock();
            if state.head.is_some() {
                return;
            }
            state.head = Some(head);
            state.head_waiter.take()
        };
        wake(waiter);
    }

    /// Stores the task, which the timers and back-pressure need.
    fn set_task(&self, task: &Retained<NSURLSessionDataTask>) {
        self.lock().task = Some(task.clone());
    }

    fn chunk(&self, bytes: Vec<u8>) {
        let waiter = {
            let mut state = self.lock();
            state.buffered += bytes.len();
            state.first_body_byte = true;
            state.chunks.push_back(bytes);
            if !state.paused && state.buffered >= BUFFER_LIMIT {
                state.paused = true;
                if let Some(task) = &state.task {
                    task.suspend();
                }
            }
            state.body_waiter.take()
        };
        wake(waiter);
    }

    /// Fails the exchange as `error` unless `still_waiting` says the step it bounds is over,
    /// and answers whether it failed; the caller cancels the task.
    fn timed_out(&self, timeout: Timeout) -> bool {
        let error = match timeout {
            Timeout::Connect => HttpError::ConnectionTimeout,
            Timeout::FirstByte => HttpError::HttpResponseTimeout,
        };
        let (failed, head_waiter, body_waiter) = {
            let mut state = self.lock();
            let waiting = !state.ended
                && state.error.is_none()
                && match timeout {
                    Timeout::Connect => state.head.is_none(),
                    Timeout::FirstByte => !state.first_body_byte,
                };
            if !waiting {
                (false, None, None)
            } else {
                state.ended = true;
                if state.head.is_none() {
                    state.head = Some(Err(error.clone()));
                }
                state.error = Some(error);
                (true, state.head_waiter.take(), state.body_waiter.take())
            }
        };
        wake(head_waiter);
        wake(body_waiter);
        failed
    }

    fn finished(&self, error: Option<HttpError>) {
        let (head_waiter, body_waiter) = {
            let mut state = self.lock();
            state.ended = true;
            if let Some(error) = error
                && state.error.is_none()
            {
                if state.head.is_none() {
                    state.head = Some(Err(error.clone()));
                }
                state.error = Some(error);
            }
            if state.head.is_none() {
                let reason = state
                    .error
                    .clone()
                    .unwrap_or(HttpError::ConnectionTerminated);
                state.head = Some(Err(reason));
            }
            (state.head_waiter.take(), state.body_waiter.take())
        };
        wake(head_waiter);
        wake(body_waiter);
    }

    /// Counts a redirect against the policy: what to do with it. One past the policy's limit
    /// fails the exchange at once, so the answer is the error rather than the redirect.
    fn redirect(&self) -> Redirect {
        let mut state = self.lock();
        match state.redirect_policy {
            RedirectPolicy::Never => Redirect::Answer,
            RedirectPolicy::Follow { .. } if state.redirects_left == 0 => {
                if state.error.is_none() && !state.ended {
                    state.ended = true;
                    if state.head.is_none() {
                        state.head = Some(Err(HttpError::TooManyRedirects));
                    }
                    state.error = Some(HttpError::TooManyRedirects);
                }
                drop(state);
                self.wake_all();
                Redirect::Fail
            }
            RedirectPolicy::Follow { .. } => {
                state.redirects_left -= 1;
                Redirect::Follow
            }
        }
    }

    /// Wakes both waiters, after a failure set by a timer or a redirect limit.
    fn wake_all(&self) {
        let (head_waiter, body_waiter) = {
            let mut state = self.lock();
            (state.head_waiter.take(), state.body_waiter.take())
        };
        wake(head_waiter);
        wake(body_waiter);
    }
}

/// What a redirect does to its exchange.
enum Redirect {
    /// The session goes on to the new location.
    Follow,
    /// The redirect response is the answer.
    Answer,
    /// The exchange has failed; its task is cancelled.
    Fail,
}

fn wake(waiter: Option<Waker>) {
    if let Some(waiter) = waiter {
        waiter.wake();
    }
}

/// Waits for the response head.
struct HeadFuture {
    exchange: Arc<Exchange>,
}

impl Future for HeadFuture {
    type Output = Result<Head, HttpError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<Head, HttpError>> {
        let mut state = self.exchange.lock();
        match state.head.take() {
            Some(head) => Poll::Ready(head),
            None => {
                state.head_waiter = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

/// The response body, read as the session delivers it.
struct HttpBodyStream {
    exchange: Arc<Exchange>,
    length: Option<u64>,
    /// Cancels the task when the body is dropped.
    _transfer: Transfer,
}

impl BodyStream for HttpBodyStream {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        Box::pin(std::future::poll_fn(move |cx| self.read_now(cx, buf)))
    }

    fn trailers(&mut self) -> BodyTrailersFuture<'_> {
        Box::pin(std::future::poll_fn(move |cx| {
            let mut state = self.exchange.lock();
            if !state.ended {
                state.body_waiter = Some(cx.waker().clone());
                return Poll::Pending;
            }
            // `NSURLSession` gives no trailers to a data delegate.
            Poll::Ready(Ok(None))
        }))
    }

    fn length(&self) -> Option<u64> {
        self.length
    }
}

impl HttpBodyStream {
    fn read_now(&mut self, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, HttpError>> {
        let mut state = self.exchange.lock();
        if let Some(chunk) = state.chunks.pop_front() {
            let read = chunk.len().min(buf.len());
            buf[..read].copy_from_slice(&chunk[..read]);
            if read < chunk.len() {
                state.chunks.push_front(chunk[read..].to_vec());
            }
            state.buffered -= read;
            if state.paused && state.buffered < BUFFER_LIMIT {
                state.paused = false;
                if let Some(task) = &state.task {
                    task.resume();
                }
            }
            return Poll::Ready(Ok(read));
        }
        if state.ended {
            return match state.error.take() {
                Some(error) => Poll::Ready(Err(error)),
                None => Poll::Ready(Ok(0)),
            };
        }
        state.body_waiter = Some(cx.waker().clone());
        Poll::Pending
    }
}

define_class!(
    /// The session's delegate: it finds each task's exchange and hands it the response head, the
    /// data, the end or the failure.
    //
    // SAFETY: `NSObject` has no subclassing requirements, and the delegate has no `Drop`.
    #[unsafe(super(NSObject))]
    #[name = "WaftHttpDelegate"]
    #[ivars = Arc<Exchanges>]
    struct HttpDelegate;

    unsafe impl NSObjectProtocol for HttpDelegate {}

    // SAFETY: `NSURLSessionDataDelegate` extends these two, which have no other requirements.
    unsafe impl NSURLSessionDelegate for HttpDelegate {}

    unsafe impl NSURLSessionTaskDelegate for HttpDelegate {
        #[unsafe(method(URLSession:task:willPerformHTTPRedirection:newRequest:completionHandler:))]
        unsafe fn will_redirect(
            &self,
            _session: &NSURLSession,
            task: &NSURLSessionTask,
            _response: &NSHTTPURLResponse,
            request: &NSURLRequest,
            completion: &DynBlock<dyn Fn(*mut NSURLRequest)>,
        ) {
            abort_on_panic(|| {
                let redirect = match self.ivars().get(task) {
                    Some(exchange) => exchange.redirect(),
                    None => Redirect::Fail,
                };
                match redirect {
                    Redirect::Follow => {
                        let request = request as *const NSURLRequest as *mut NSURLRequest;
                        completion.call((request,));
                    }
                    Redirect::Answer => completion.call((std::ptr::null_mut(),)),
                    Redirect::Fail => {
                        task.cancel();
                        completion.call((std::ptr::null_mut(),));
                    }
                }
            });
        }

        #[unsafe(method(URLSession:task:didCompleteWithError:))]
        fn completed(
            &self,
            _session: &NSURLSession,
            task: &NSURLSessionTask,
            error: Option<&NSError>,
        ) {
            abort_on_panic(|| {
                let Some(exchange) = self.ivars().get(task) else {
                    return;
                };
                exchange.finished(error.map(error_of));
            });
        }
    }

    unsafe impl NSURLSessionDataDelegate for HttpDelegate {
        #[unsafe(method(URLSession:dataTask:didReceiveResponse:completionHandler:))]
        unsafe fn received_response(
            &self,
            _session: &NSURLSession,
            task: &NSURLSessionDataTask,
            response: &NSURLResponse,
            completion: &DynBlock<dyn Fn(NSURLSessionResponseDisposition)>,
        ) {
            abort_on_panic(|| {
                if let Some(exchange) = self.ivars().get(task) {
                    exchange.responded(head_of(response));
                }
                completion.call((NSURLSessionResponseDisposition::Allow,));
            });
        }

        #[unsafe(method(URLSession:dataTask:didReceiveData:))]
        fn received_data(
            &self,
            _session: &NSURLSession,
            task: &NSURLSessionDataTask,
            data: &NSData,
        ) {
            abort_on_panic(|| {
                if let Some(exchange) = self.ivars().get(task) {
                    exchange.chunk(data.to_vec());
                }
            });
        }
    }
);

impl HttpDelegate {
    fn new(exchanges: Arc<Exchanges>) -> Retained<HttpDelegate> {
        let this = HttpDelegate::alloc().set_ivars(exchanges);
        // SAFETY: plain NSObject initialisation of an allocated instance.
        unsafe { msg_send![super(this), init] }
    }
}

fn head_of(response: &NSURLResponse) -> Result<Head, HttpError> {
    let Some(response) = response.downcast_ref::<NSHTTPURLResponse>() else {
        return Err(HttpError::HttpProtocolError);
    };
    let status = StatusCode::from_u16(u16::try_from(response.statusCode()).unwrap_or(0))
        .map_err(|_| HttpError::HttpProtocolError)?;
    let all = response.allHeaderFields();
    // SAFETY: the dictionary is the response's header fields, which are strings.
    let fields: &objc2_foundation::NSDictionary<NSString, NSString> =
        unsafe { all.cast_unchecked() };
    let mut headers = HeaderMap::new();
    for name in fields.allKeys().iter() {
        let Some(value) = fields.objectForKey(&name) else {
            continue;
        };
        let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.to_string().as_bytes()),
            HeaderValue::from_bytes(value.to_string().as_bytes()),
        ) else {
            continue;
        };
        headers.append(name, value);
    }
    // The session decompresses a body itself, so a compressed body's `Content-Length` counts
    // bytes the app never sees.
    let compressed = headers
        .get(http::header::CONTENT_ENCODING)
        .is_some_and(|encoding| !encoding.as_bytes().eq_ignore_ascii_case(b"identity"));
    let length = if compressed {
        None
    } else {
        headers
            .get(http::header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok())
    };
    Ok(Head {
        status,
        version: Version::HTTP_11,
        headers,
        length,
    })
}

/// What a timer in the client bounds.
#[derive(Clone, Copy)]
enum Timeout {
    /// From the task's start until the response head.
    Connect,
    /// From the response head until the first body byte.
    FirstByte,
}

/// Starts `timeout` for `exchange`: when it fires, the exchange fails unless its step is over.
fn start_timer(
    exchange: Arc<Exchange>,
    task: Retained<NSURLSessionDataTask>,
    timeout: Duration,
    which: Timeout,
) {
    let when = DispatchTime::try_from(timeout).unwrap_or(DispatchTime::FOREVER);
    let queue = DispatchQueue::global_queue(GlobalQueueIdentifier::Priority(
        DispatchQueueGlobalPriority::Default,
    ));
    let _ = queue.after(when, move || {
        if exchange.timed_out(which) {
            task.cancel();
        }
    });
}

/// Why `NSURLSession` failed, in WASI's terms. A task this client cancels has no one waiting any
/// more, or has failed already; one cancelled by anyone else ends short, which is an error.
#[allow(non_upper_case_globals, reason = "the codes keep Foundation's names")]
fn error_of(error: &NSError) -> HttpError {
    let code = error.code();
    let reason = error.localizedDescription().to_string();
    match code {
        NSURLErrorTimedOut => HttpError::HttpResponseTimeout,
        NSURLErrorBadURL | NSURLErrorUnsupportedURL => HttpError::HttpRequestUriInvalid,
        NSURLErrorCannotFindHost | NSURLErrorDNSLookupFailed => {
            HttpError::DnsError(DnsErrorPayload {
                rcode: Some(reason),
                info_code: None,
            })
        }
        NSURLErrorCannotConnectToHost => HttpError::ConnectionRefused,
        NSURLErrorNetworkConnectionLost | NSURLErrorCancelled => HttpError::ConnectionTerminated,
        NSURLErrorNotConnectedToInternet
        | NSURLErrorDataNotAllowed
        | NSURLErrorInternationalRoamingOff
        | NSURLErrorCallIsActive
        | NSURLErrorResourceUnavailable
        | NSURLErrorCannotLoadFromNetwork => HttpError::DestinationUnavailable,
        NSURLErrorHTTPTooManyRedirects => HttpError::TooManyRedirects,
        NSURLErrorRedirectToNonExistentLocation => HttpError::DestinationNotFound,
        NSURLErrorSecureConnectionFailed => HttpError::TlsProtocolError,
        NSURLErrorServerCertificateHasBadDate
        | NSURLErrorServerCertificateUntrusted
        | NSURLErrorServerCertificateHasUnknownRoot
        | NSURLErrorServerCertificateNotYetValid
        | NSURLErrorClientCertificateRejected
        | NSURLErrorClientCertificateRequired
        | NSURLErrorAppTransportSecurityRequiresSecureConnection => HttpError::TlsCertificateError,
        NSURLErrorUserCancelledAuthentication | NSURLErrorUserAuthenticationRequired => {
            HttpError::HttpRequestDenied
        }
        NSURLErrorBadServerResponse
        | NSURLErrorCannotParseResponse
        | NSURLErrorZeroByteResource
        | NSURLErrorCannotDecodeRawData
        | NSURLErrorCannotDecodeContentData => HttpError::HttpProtocolError,
        _ => HttpError::Internal(Some(reason)),
    }
}
