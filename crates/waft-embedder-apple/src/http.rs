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
#![allow(unsafe_code)]

use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

use block2::DynBlock;
use http::{HeaderMap, HeaderName, HeaderValue, Request, Response, StatusCode, Uri, Version};
use objc2::rc::Retained;
use objc2::runtime::{NSObject, NSObjectProtocol, ProtocolObject};
use objc2::{AllocAnyThread, DefinedClass, define_class, msg_send};
use objc2_foundation::{
    NSData, NSError, NSHTTPURLResponse, NSMutableURLRequest, NSOperationQueue, NSString, NSURL,
    NSURLErrorCancelled, NSURLErrorCannotConnectToHost, NSURLErrorCannotFindHost,
    NSURLErrorNetworkConnectionLost, NSURLErrorNotConnectedToInternet,
    NSURLErrorSecureConnectionFailed, NSURLErrorServerCertificateUntrusted, NSURLErrorTimedOut,
    NSURLRequest, NSURLResponse, NSURLSession, NSURLSessionDataDelegate, NSURLSessionDataTask,
    NSURLSessionDelegate, NSURLSessionResponseDisposition, NSURLSessionTask,
    NSURLSessionTaskDelegate,
};
use waft_embedder::{
    Body, BodyReadFuture, BodyStream, BodyTrailersFuture, HttpClient, HttpError, HttpFuture,
    RedirectPolicy, SendOptions,
};

use crate::abort_on_panic;

/// Waft's HTTP client on Apple's systems: one `NSURLSession` for the app, whose delegate routes
/// each task's events to the exchange waiting for them.
pub struct AppleHttpClient {
    session: Retained<NSURLSession>,
    exchanges: Arc<Exchanges>,
    /// Kept because the session's reference to its delegate is the one that matters, and this
    /// keeps it alive for the session's life.
    _delegate: Retained<HttpDelegate>,
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
            answer
                .body(body)
                .map_err(|error| HttpError::Protocol(error.to_string()))
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
    if let Some(timeout) = options.timeout {
        request.setTimeoutInterval(timeout.as_secs_f64());
    }
    for (name, value) in &headers {
        if let (Ok(name), Ok(value)) = (
            HeaderName::from_bytes(name.as_str().as_bytes()),
            HeaderValue::from_bytes(value.as_bytes()),
        ) {
            request.setValue_forHTTPHeaderField(
                Some(&NSString::from_str(value.to_str().unwrap_or_default())),
                &NSString::from_str(name.as_str()),
            );
        }
    }
    if !bytes.is_empty() {
        request.setHTTPBody(Some(&NSData::from_vec(bytes)));
    }
    let task = session.dataTaskWithRequest(&request);
    exchanges
        .lock()
        .insert(task.taskIdentifier(), Arc::clone(&exchange));
    task.resume();
    Ok(Transfer { task, exchanges })
}

fn url_of(uri: &Uri) -> Result<Retained<NSURL>, HttpError> {
    let scheme = uri.scheme_str();
    if !matches!(scheme, Some("http") | Some("https")) || uri.host().is_none() {
        return Err(HttpError::InvalidUrl);
    }
    NSURL::URLWithString(&NSString::from_str(&uri.to_string())).ok_or(HttpError::InvalidUrl)
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
    body_waiter: Option<Waker>,
    ended: bool,
    error: Option<HttpError>,
    redirects_left: usize,
    redirect_policy: RedirectPolicy,
}

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
                body_waiter: None,
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

    fn chunk(&self, bytes: Vec<u8>) {
        let waiter = {
            let mut state = self.lock();
            state.chunks.push_back(bytes);
            state.body_waiter.take()
        };
        wake(waiter);
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
                let reason = state.error.clone().unwrap_or_else(|| {
                    HttpError::Connection("the response ended without a head".to_owned())
                });
                state.head = Some(Err(reason));
            }
            (state.head_waiter.take(), state.body_waiter.take())
        };
        wake(head_waiter);
        wake(body_waiter);
    }

    /// Counts a redirect against the policy: whether it may be followed.
    fn follow_redirect(&self) -> bool {
        let mut state = self.lock();
        match state.redirect_policy {
            RedirectPolicy::Never => false,
            RedirectPolicy::Follow { .. } if state.redirects_left == 0 => {
                if state.error.is_none() {
                    state.error = Some(HttpError::TooManyRedirects);
                }
                false
            }
            RedirectPolicy::Follow { .. } => {
                state.redirects_left -= 1;
                true
            }
        }
    }
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
                let follow = self
                    .ivars()
                    .get(task)
                    .is_some_and(|exchange| exchange.follow_redirect());
                if follow {
                    let request = request as *const NSURLRequest as *mut NSURLRequest;
                    completion.call((request,));
                } else {
                    let failed = self
                        .ivars()
                        .get(task)
                        .is_some_and(|exchange| {
                            exchange.lock().error == Some(HttpError::TooManyRedirects)
                        });
                    if failed {
                        task.cancel();
                    }
                    // Nothing follows: the redirect response is answered as it is.
                    completion.call((std::ptr::null_mut(),));
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
                let error = error.and_then(error_of);
                exchange.finished(error);
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
        return Err(HttpError::Protocol(
            "the response is not an HTTP response".to_owned(),
        ));
    };
    let status = StatusCode::from_u16(u16::try_from(response.statusCode()).unwrap_or(0))
        .map_err(|_| HttpError::Protocol("the status is not an HTTP status".to_owned()))?;
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

/// Why `NSURLSession` failed, in Waft's terms.
fn error_of(error: &NSError) -> Option<HttpError> {
    let code = error.code();
    if code == NSURLErrorCancelled {
        return None;
    }
    let reason = error.localizedDescription().to_string();
    if code == NSURLErrorTimedOut {
        return Some(HttpError::Timeout);
    }
    if code == NSURLErrorCannotConnectToHost
        || code == NSURLErrorCannotFindHost
        || code == NSURLErrorNotConnectedToInternet
    {
        return Some(HttpError::Refused(reason));
    }
    if code == NSURLErrorSecureConnectionFailed || code == NSURLErrorServerCertificateUntrusted {
        return Some(HttpError::Tls(reason));
    }
    if code == NSURLErrorNetworkConnectionLost {
        return Some(HttpError::Connection(reason));
    }
    Some(HttpError::Connection(reason))
}
