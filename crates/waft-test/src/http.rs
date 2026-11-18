//! HTTP in memory: a client that answers the responses a test configured, keeps each request it
//! was given, and streams a response's body a chunk at a time so progress can be watched. Every
//! clone is the same client.

use std::cell::RefCell;
use std::rc::Rc;

use http::{HeaderMap, Method, Request, Response, StatusCode, Uri};
use waft_embedder::{
    Body, BodyReadFuture, BodyStream, BodyTrailersFuture, HttpClient, HttpError, HttpFuture,
    SendOptions,
};

/// A client that answers what the test configured. A request for a URL with no answer fails as a
/// connection that found nothing.
#[derive(Clone, Default)]
pub struct MockHttpClient(Rc<ClientState>);

#[derive(Default)]
struct ClientState {
    responses: RefCell<Vec<(String, MockResponse)>>,
    asked: RefCell<Vec<SentRequest>>,
}

/// One response a [`MockHttpClient`] answers.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MockResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    /// The body in the pieces it arrives in, one per read.
    pub chunks: Vec<Vec<u8>>,
    /// Fields that come after the body.
    pub trailers: Option<HeaderMap>,
    /// The length the sender claims; the sum of the chunks when not given.
    pub length: Option<u64>,
    /// Why the body stopped short instead of ending.
    pub error: Option<HttpError>,
}

/// A request the app sent, as the client kept it.
#[derive(Clone, Debug, PartialEq)]
pub struct SentRequest {
    pub method: Method,
    pub uri: Uri,
    pub headers: HeaderMap,
    pub body: Vec<u8>,
    pub options: SendOptions,
}

impl MockHttpClient {
    pub fn new() -> MockHttpClient {
        MockHttpClient::default()
    }

    /// Answers every request for `url` with `response`. The last answer for a URL is the one
    /// used.
    pub fn answering(self, url: impl Into<String>, response: MockResponse) -> MockHttpClient {
        self.0.responses.borrow_mut().push((url.into(), response));
        self
    }

    /// Every request the client was given, in order.
    pub fn asked(&self) -> Vec<SentRequest> {
        self.0.asked.borrow().clone()
    }

    fn response_for(&self, uri: &Uri) -> Option<MockResponse> {
        self.0
            .responses
            .borrow()
            .iter()
            .rev()
            .find(|(url, _)| url == &uri.to_string())
            .map(|(_, response)| response.clone())
    }

    fn keep(&self, request: SentRequest) {
        self.0.asked.borrow_mut().push(request);
    }
}

impl MockResponse {
    /// An `OK` response with no body.
    pub fn new() -> MockResponse {
        MockResponse {
            status: StatusCode::OK,
            ..MockResponse::default()
        }
    }

    pub fn status(mut self, status: StatusCode) -> MockResponse {
        self.status = status;
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> MockResponse {
        self.headers.append(
            name.parse::<http::HeaderName>().expect("a header name"),
            value.parse().expect("a header value"),
        );
        self
    }

    /// A body that arrives as one piece.
    pub fn body(mut self, bytes: impl Into<Vec<u8>>) -> MockResponse {
        self.chunks.push(bytes.into());
        self
    }

    /// A body that arrives in `chunks`, one per read.
    pub fn chunks(mut self, chunks: Vec<Vec<u8>>) -> MockResponse {
        self.chunks = chunks;
        self
    }

    pub fn trailers(mut self, trailers: HeaderMap) -> MockResponse {
        self.trailers = Some(trailers);
        self
    }

    /// A length other than the bytes actually sent, as a compressed body reports.
    pub fn length(mut self, length: u64) -> MockResponse {
        self.length = Some(length);
        self
    }

    /// The body fails after the chunks it was given, with `error`.
    pub fn error(mut self, error: HttpError) -> MockResponse {
        self.error = Some(error);
        self
    }
}

impl HttpClient for MockHttpClient {
    fn send(&self, request: Request<Body>, options: SendOptions) -> HttpFuture {
        let mut request = request;
        let response = self.response_for(request.uri());
        let kept = self.clone();
        let method = request.method().clone();
        let uri = request.uri().clone();
        let headers = request.headers().clone();
        Box::pin(async move {
            let body = request.body_mut();
            let bytes = body.read_all().await?;
            kept.keep(SentRequest {
                method,
                uri: uri.clone(),
                headers,
                body: bytes,
                options,
            });
            let response = response.ok_or_else(|| {
                HttpError::Internal(Some(format!("no response is configured for {uri}")))
            })?;
            let mut answer = Response::builder().status(response.status);
            for (name, value) in &response.headers {
                answer = answer.header(name, value);
            }
            let body = Body::from_stream(MockBodyStream::new(response));
            answer.body(body).map_err(|_| HttpError::HttpProtocolError)
        })
    }
}

/// A configured response's body, read one chunk per read.
struct MockBodyStream {
    chunks: std::vec::IntoIter<Vec<u8>>,
    current: Vec<u8>,
    read: usize,
    trailers: Option<HeaderMap>,
    length: Option<u64>,
    error: Option<HttpError>,
}

impl MockBodyStream {
    fn new(response: MockResponse) -> MockBodyStream {
        let length = response
            .length
            .or_else(|| Some(response.chunks.iter().map(Vec::len).sum::<usize>() as u64));
        MockBodyStream {
            chunks: response.chunks.into_iter(),
            current: Vec::new(),
            read: 0,
            trailers: response.trailers,
            length,
            error: response.error,
        }
    }
}

impl BodyStream for MockBodyStream {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        if self.read == self.current.len() {
            match self.chunks.next() {
                Some(chunk) => {
                    self.current = chunk;
                    self.read = 0;
                }
                None => {
                    let answer = match self.error.take() {
                        Some(error) => Err(error),
                        None => Ok(0),
                    };
                    return Box::pin(std::future::ready(answer));
                }
            }
        }
        let read = (self.current.len() - self.read).min(buf.len());
        buf[..read].copy_from_slice(&self.current[self.read..self.read + read]);
        self.read += read;
        Box::pin(std::future::ready(Ok(read)))
    }

    fn trailers(&mut self) -> BodyTrailersFuture<'_> {
        Box::pin(std::future::ready(Ok(self.trailers.take())))
    }

    fn length(&self) -> Option<u64> {
        self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;
    use std::task::Poll;

    fn now<T>(future: impl Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        loop {
            if let Poll::Ready(value) = future.as_mut().poll(&mut context) {
                return value;
            }
        }
    }

    #[test]
    fn a_configured_response_answers_with_its_head_and_streams_its_body() {
        let client = MockHttpClient::new().answering(
            "https://example.test/photo.png",
            MockResponse::new()
                .header("content-type", "image/png")
                .chunks(vec![b"one".to_vec(), b"two".to_vec()])
                .trailers(HeaderMap::new()),
        );
        let request = Request::builder()
            .uri("https://example.test/photo.png")
            .body(Body::empty())
            .unwrap();
        let mut response = now(client.send(request, SendOptions::default())).unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "image/png");
        assert_eq!(
            now(response.body_mut().read_all()).unwrap(),
            b"onetwo".to_vec()
        );
        assert_eq!(client.asked().len(), 1);
        assert_eq!(
            client.asked()[0].uri,
            "https://example.test/photo.png".parse::<Uri>().unwrap()
        );
    }

    #[test]
    fn a_url_with_no_answer_fails_as_a_connection_that_found_nothing() {
        let client = MockHttpClient::new();
        let request = Request::builder()
            .uri("https://example.test/missing")
            .body(Body::empty())
            .unwrap();
        let error = now(client.send(request, SendOptions::default())).unwrap_err();
        assert!(matches!(error, HttpError::Internal(_)));
    }
}
