//! The Apple HTTP client against a server on this machine: a body arriving in pieces, a redirect
//! followed and one refused, too many redirects, a request with a body and repeated headers, a
//! timeout, and a URL nothing listens on.

// The local server runs on a thread, as a test's may.
#![allow(clippy::disallowed_methods)]
// The crate is empty off Apple's systems, and so is this.
#![cfg(any(target_os = "macos", target_os = "ios"))]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use http::{HeaderMap, Request, StatusCode};
use waft_embedder::{
    Body, BodyReadFuture, BodyStream, BodyTrailersFuture, HttpClient, HttpError, RedirectPolicy,
    SendOptions,
};
use waft_embedder_apple::http::AppleHttpClient;

/// The path a request asked for, after reading its head and body.
fn read_request(stream: &mut TcpStream) -> String {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        match stream.read(&mut byte) {
            Ok(1) => head.push(byte[0]),
            _ => return String::new(),
        }
    }
    let text = String::from_utf8_lossy(&head);
    let first = text.lines().next().unwrap_or_default();
    first.split(' ').nth(1).unwrap_or_default().to_owned()
}

fn response(status: &str, headers: &[(&str, &str)], body: &[u8]) -> Vec<u8> {
    let mut answer = format!("HTTP/1.1 {status}\r\n");
    for (name, value) in headers {
        answer.push_str(&format!("{name}: {value}\r\n"));
    }
    answer.push_str(&format!("Content-Length: {}\r\n", body.len()));
    answer.push_str("Connection: close\r\n\r\n");
    let mut bytes = answer.into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn ok(body: &[u8]) -> Vec<u8> {
    response("200 OK", &[], body)
}

/// The method and path a request asked for and the bytes its body carried, after reading all
/// three.
fn read_request_and_body(stream: &mut TcpStream) -> (String, String, Vec<u8>) {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 1024];
    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => bytes.extend_from_slice(&buffer[..read]),
            _ => break,
        }
    }
    let head = String::from_utf8_lossy(&bytes).into_owned();
    let mut asked = head.lines().next().unwrap_or_default().split(' ');
    let method = asked.next().unwrap_or_default().to_owned();
    let path = asked.next().unwrap_or_default().to_owned();
    let length: usize = head
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(0);
    while bytes.len() < head.len() + length {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => bytes.extend_from_slice(&buffer[..read]),
            _ => break,
        }
    }
    (method, path, bytes[head.len()..].to_vec())
}

/// A body of bytes that streams, whose bytes are gone once they are read: one a client cannot
/// send again.
struct Once(Vec<u8>);

impl BodyStream for Once {
    fn read<'a>(&'a mut self, buf: &'a mut [u8]) -> BodyReadFuture<'a> {
        let read = self.0.len().min(buf.len());
        buf[..read].copy_from_slice(&self.0);
        self.0.clear();
        Box::pin(std::future::ready(Ok(read)))
    }

    fn trailers(&mut self) -> BodyTrailersFuture<'_> {
        Box::pin(std::future::ready(Ok(None)))
    }

    fn length(&self) -> Option<u64> {
        None
    }
}

fn get(url: &str) -> Request<Body> {
    Request::builder()
        .uri(url)
        .body(Body::empty())
        .expect("a request")
}

#[test]
fn a_body_arrives_in_pieces_and_reads_whole() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().expect("the port's address");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("a connection");
        read_request(&mut stream);
        let head = b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: close\r\n\r\n";
        stream.write_all(head).unwrap();
        for piece in [b"hello ".as_slice(), b"chun", b"ks"] {
            stream.write_all(piece).unwrap();
            stream.flush().unwrap();
            thread::sleep(Duration::from_millis(30));
        }
    });
    let client = AppleHttpClient::new();
    let mut response = pollster::block_on(client.send(
        get(&format!("http://{address}/chunks")),
        SendOptions::default(),
    ))
    .expect("the response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.body_mut().length(),
        Some(12),
        "the head's Content-Length is the body's length"
    );
    let body = pollster::block_on(response.body_mut().read_all()).expect("the body");
    assert_eq!(body, b"hello chunks");
}

#[test]
fn a_redirect_is_followed_or_answered_as_it_is() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    let location = format!("http://{address}/final");
    let answer_location = location.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let path = read_request(&mut stream);
            let answer = if path == "/redirect" {
                response("302 Found", &[("Location", &answer_location)], b"")
            } else {
                ok(b"final")
            };
            let _ = stream.write_all(&answer);
        }
    });

    let client = AppleHttpClient::new();
    let response = pollster::block_on(client.send(
        get(&format!("http://{address}/redirect")),
        SendOptions::default(),
    ))
    .expect("the response");
    assert_eq!(response.status(), StatusCode::OK);
    let mut response = response;
    assert_eq!(
        pollster::block_on(response.body_mut().read_all()).unwrap(),
        b"final"
    );

    let response = pollster::block_on(client.send(
        get(&format!("http://{address}/redirect")),
        SendOptions::default().redirects(RedirectPolicy::Never),
    ))
    .expect("the redirect itself");
    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(response.headers()["location"].to_str().unwrap(), location);
}

#[test]
fn a_307_redirect_resends_the_request_body() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    let location = format!("http://{address}/final");
    let resent = Arc::new(Mutex::new(None::<(String, String, Vec<u8>)>));
    let kept = Arc::clone(&resent);
    let answer_location = location.clone();
    thread::spawn(move || {
        for (seen, stream) in listener.incoming().enumerate() {
            let Ok(mut stream) = stream else {
                continue;
            };
            let (method, path, body) = read_request_and_body(&mut stream);
            let answer = if seen == 0 {
                response(
                    "307 Temporary Redirect",
                    &[("Location", &answer_location)],
                    b"",
                )
            } else {
                *kept.lock().unwrap() = Some((method, path, body));
                ok(b"final")
            };
            let _ = stream.write_all(&answer);
        }
    });

    let client = AppleHttpClient::new();
    let request = Request::builder()
        .method("POST")
        .uri(format!("http://{address}/redirect"))
        .body(Body::from(b"resend me".to_vec()))
        .unwrap();
    let mut response =
        pollster::block_on(client.send(request, SendOptions::default())).expect("the response");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        pollster::block_on(response.body_mut().read_all()).unwrap(),
        b"final"
    );
    let (method, path, body) = resent
        .lock()
        .unwrap()
        .clone()
        .expect("the request was resent");
    assert_eq!(method, "POST", "a 307 keeps the method");
    assert_eq!(path, "/final");
    assert_eq!(
        body, b"resend me",
        "the redirect carried the request's body again"
    );
}

#[test]
fn a_307_of_a_body_that_cannot_be_resent_is_answered_as_it_is() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    let location = format!("http://{address}/final");
    let answer_location = location.clone();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            read_request_and_body(&mut stream);
            let answer = response(
                "307 Temporary Redirect",
                &[("Location", &answer_location)],
                b"",
            );
            let _ = stream.write_all(&answer);
        }
    });

    let client = AppleHttpClient::new();
    let request = Request::builder()
        .method("POST")
        .uri(format!("http://{address}/redirect"))
        .body(Body::from_stream(Once(b"streamed".to_vec())))
        .unwrap();
    let mut response =
        pollster::block_on(client.send(request, SendOptions::default())).expect("the response");
    assert_eq!(
        response.status(),
        StatusCode::TEMPORARY_REDIRECT,
        "a body that cannot be sent again is not sent half"
    );
    assert_eq!(response.headers()["location"].to_str().unwrap(), location);
    assert_eq!(
        pollster::block_on(response.body_mut().read_all()).unwrap(),
        b""
    );
}

#[test]
fn a_request_with_trailers_fails_where_the_session_cannot_send_them() {
    let mut trailers = HeaderMap::new();
    trailers.insert("x-checksum", 7u32.into());
    let client = AppleHttpClient::new();
    let request = Request::builder()
        .method("POST")
        .uri("http://127.0.0.1:1/put")
        .body(Body::from(b"the bytes".to_vec()).with_trailers(trailers))
        .unwrap();
    let error = pollster::block_on(client.send(request, SendOptions::default())).unwrap_err();
    assert_eq!(
        error,
        HttpError::Internal(Some("NSURLSession cannot send trailers".to_owned()))
    );
}

#[test]
fn a_policy_that_allows_fewer_redirects_than_the_server_sends_fails() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    let location = format!("http://{address}/loop");
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else {
                continue;
            };
            read_request(&mut stream);
            let answer = response("302 Found", &[("Location", &location)], b"");
            let _ = stream.write_all(&answer);
        }
    });
    let client = AppleHttpClient::new();
    let error = pollster::block_on(client.send(
        get(&format!("http://{address}/loop")),
        SendOptions::default().redirects(RedirectPolicy::Follow { max: 2 }),
    ))
    .unwrap_err();
    assert_eq!(error, HttpError::TooManyRedirects);
}

#[test]
fn a_request_carries_its_method_headers_and_body() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    let received = Arc::new(Mutex::new(String::new()));
    let kept = Arc::clone(&received);
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("a connection");
        let mut bytes = Vec::new();
        let mut buffer = [0u8; 1024];
        while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
            let read = stream.read(&mut buffer).unwrap();
            bytes.extend_from_slice(&buffer[..read]);
        }
        let head = String::from_utf8_lossy(&bytes).into_owned();
        let length: usize = head
            .lines()
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .and_then(|value| value.trim().parse().ok())
            .unwrap_or(0);
        while bytes.len() < head.len() + length {
            let read = stream.read(&mut buffer).unwrap();
            bytes.extend_from_slice(&buffer[..read]);
        }
        *kept.lock().unwrap() = String::from_utf8_lossy(&bytes).into_owned();
        stream.write_all(&ok(b"posted")).unwrap();
    });
    let client = AppleHttpClient::new();
    let request = Request::builder()
        .method("POST")
        .uri(format!("http://{address}/echo"))
        .header("x-test", "yes")
        .header("x-repeated", "one")
        .header("x-repeated", "two")
        .body(Body::from(b"a body".to_vec()))
        .unwrap();
    let mut response = pollster::block_on(client.send(request, SendOptions::default())).unwrap();
    assert_eq!(
        pollster::block_on(response.body_mut().read_all()).unwrap(),
        b"posted"
    );
    let received = received.lock().unwrap().clone();
    assert!(received.starts_with("POST /echo HTTP/1.1"), "{received}");
    assert!(received.contains("x-test: yes"), "{received}");
    assert!(
        received.contains("x-repeated: one,two") || received.contains("x-repeated: one, two"),
        "a repeated header sends every value: {received}"
    );
    assert!(received.ends_with("a body"), "{received}");
}

#[test]
fn a_connection_that_never_gets_a_head_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("a connection");
        read_request(&mut stream);
        thread::sleep(Duration::from_secs(2));
    });
    let client = AppleHttpClient::new();
    let error = pollster::block_on(client.send(
        get(&format!("http://{address}/silent")),
        SendOptions::default().connect_timeout(Duration::from_millis(200)),
    ))
    .unwrap_err();
    assert_eq!(error, HttpError::ConnectionTimeout);
}

#[test]
fn a_server_that_never_sends_the_first_body_byte_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("a connection");
        read_request(&mut stream);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n")
            .unwrap();
        stream.flush().unwrap();
        thread::sleep(Duration::from_secs(2));
        let _ = stream.write_all(b"ok");
    });
    let client = AppleHttpClient::new();
    // `NSURLSession` reports the head and body together for a body this small, so the timeout
    // fails the exchange rather than only its body.
    let error = pollster::block_on(client.send(
        get(&format!("http://{address}/late-first-byte")),
        SendOptions::default().first_byte_timeout(Duration::from_secs(1)),
    ))
    .unwrap_err();
    assert_eq!(error, HttpError::HttpResponseTimeout);
}

#[test]
fn a_body_that_pauses_between_bytes_times_out() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("a connection");
        read_request(&mut stream);
        stream
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n")
            .unwrap();
        stream.write_all(b"o").unwrap();
        stream.flush().unwrap();
        thread::sleep(Duration::from_secs(3));
        let _ = stream.write_all(b"k");
    });
    let client = AppleHttpClient::new();
    let error = pollster::block_on(client.send(
        get(&format!("http://{address}/slow-body")),
        SendOptions::default().between_bytes_timeout(Duration::from_secs(1)),
    ))
    .unwrap_err();
    assert_eq!(error, HttpError::HttpResponseTimeout);
}

#[test]
fn a_body_larger_than_the_buffer_reads_whole_in_order() {
    const LEN: usize = 1024 * 1024;
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("a connection");
        read_request(&mut stream);
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {LEN}\r\n\r\n");
        stream.write_all(head.as_bytes()).unwrap();
        stream.flush().unwrap();
        let piece: Vec<u8> = (0..64 * 1024).map(|byte| byte as u8).collect();
        for _ in 0..LEN / piece.len() {
            stream.write_all(&piece).unwrap();
        }
    });
    let client = AppleHttpClient::new();
    let mut response = pollster::block_on(client.send(
        get(&format!("http://{address}/large")),
        SendOptions::default(),
    ))
    .expect("the head arrives");
    let mut body = Vec::with_capacity(LEN);
    let mut chunk = vec![0u8; 4 * 1024];
    loop {
        let read = pollster::block_on(response.body_mut().read(&mut chunk)).unwrap();
        if read == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..read]);
    }
    assert_eq!(body.len(), LEN);
    assert!(
        body.iter()
            .enumerate()
            .all(|(index, byte)| *byte as usize == index % 256)
    );
}

#[test]
fn a_url_no_one_listens_on_is_refused() {
    let address = {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port");
        listener.local_addr().unwrap()
    };
    let client = AppleHttpClient::new();
    let error = pollster::block_on(client.send(
        get(&format!("http://{address}/gone")),
        SendOptions::default(),
    ))
    .unwrap_err();
    assert_eq!(error, HttpError::ConnectionRefused);
}

#[test]
fn a_url_that_is_not_http_is_invalid() {
    let client = AppleHttpClient::new();
    let error =
        pollster::block_on(client.send(get("ftp://example.test/a"), SendOptions::default()))
            .unwrap_err();
    assert_eq!(error, HttpError::HttpRequestUriInvalid);
}
