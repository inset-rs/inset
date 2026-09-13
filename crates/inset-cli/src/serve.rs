//! A static file server for `run -d web`: one folder, plain HTTP, no dependencies.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Browser {
    Default,
    Chrome,
}

pub fn serve_and_open(dir: PathBuf, browser: Browser) -> Result<()> {
    let listener = bind()?;
    let url = format!("http://127.0.0.1:{}/", listener.local_addr()?.port());
    println!("serving {} at {url}", dir.display());
    println!("press ctrl-c to stop");
    open(&url, browser);
    for stream in listener.incoming() {
        let stream = stream?;
        let dir = dir.clone();
        std::thread::spawn(move || handle(stream, &dir));
    }
    Ok(())
}

fn bind() -> Result<TcpListener> {
    (8000..8100)
        .find_map(|port| TcpListener::bind(("127.0.0.1", port)).ok())
        .context("no free port between 8000 and 8099")
}

fn open(url: &str, browser: Browser) {
    let mut command = match (browser, std::env::consts::OS) {
        (Browser::Chrome, "macos") => {
            let mut c = Command::new("open");
            c.args(["-a", "Google Chrome", url]);
            c
        }
        (Browser::Chrome, "windows") => {
            let mut c = Command::new("cmd");
            c.args(["/C", "start", "chrome", url]);
            c
        }
        (Browser::Chrome, _) => {
            let mut c = Command::new("google-chrome");
            c.arg(url);
            c
        }
        (Browser::Default, "macos") => {
            let mut c = Command::new("open");
            c.arg(url);
            c
        }
        (Browser::Default, "windows") => {
            let mut c = Command::new("cmd");
            c.args(["/C", "start", "", url]);
            c
        }
        (Browser::Default, _) => {
            let mut c = Command::new("xdg-open");
            c.arg(url);
            c
        }
    };
    if command.status().is_err() {
        eprintln!("note: could not open a browser; visit {url}");
    }
}

fn handle(mut stream: TcpStream, dir: &Path) {
    let Some(request) = read_request(&mut stream) else {
        return;
    };
    let Some((path, accepts_brotli)) = parse(&request) else {
        let _ = respond(
            &mut stream,
            "400 Bad Request",
            "text/plain",
            &[],
            b"bad request",
        );
        return;
    };
    let Some(file) = resolve(dir, &path) else {
        let _ = respond(
            &mut stream,
            "404 Not Found",
            "text/plain",
            &[],
            b"not found",
        );
        return;
    };
    let content_type = content_type(&file);
    let mut compressed = file.as_os_str().to_owned();
    compressed.push(".br");
    let compressed = PathBuf::from(compressed);
    let (body, extra) = if accepts_brotli && compressed.is_file() {
        (std::fs::read(&compressed), &["Content-Encoding: br"][..])
    } else {
        (std::fs::read(&file), &[][..])
    };
    match body {
        Ok(body) => {
            let _ = respond(&mut stream, "200 OK", content_type, extra, &body);
        }
        Err(_) => {
            let _ = respond(
                &mut stream,
                "500 Internal Server Error",
                "text/plain",
                &[],
                b"read failed",
            );
        }
    }
}

fn read_request(stream: &mut TcpStream) -> Option<String> {
    let mut bytes = Vec::new();
    let mut chunk = [0u8; 1024];
    while !bytes.windows(4).any(|w| w == b"\r\n\r\n") && bytes.len() < 16 * 1024 {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
    }
    String::from_utf8(bytes).ok()
}

/// The request path (query stripped, percent-decoded) and whether `br` is acceptable.
fn parse(request: &str) -> Option<(String, bool)> {
    let mut lines = request.lines();
    let mut first = lines.next()?.split_whitespace();
    if first.next()? != "GET" {
        return None;
    }
    let target = first.next()?;
    let path = percent_decode(target.split('?').next()?);
    let accepts_brotli = lines.any(|line| {
        let (name, value) = line.split_once(':').unwrap_or(("", ""));
        name.eq_ignore_ascii_case("accept-encoding") && value.split(',').any(|v| v.trim() == "br")
    });
    Some((path, accepts_brotli))
}

fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = text.get(i + 1..i + 3)
            && let Ok(value) = u8::from_str_radix(hex, 16)
        {
            out.push(value);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// A file under `dir`; `/` and directories mean their `index.html`. Never above `dir`.
fn resolve(dir: &Path, path: &str) -> Option<PathBuf> {
    let mut file = dir.to_path_buf();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => return None,
            other => file.push(other),
        }
    }
    if file.is_dir() {
        file.push("index.html");
    }
    file.is_file().then_some(file)
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") | Some("mjs") => "text/javascript",
        Some("wasm") => "application/wasm",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn respond(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    extra: &[&str],
    body: &[u8],
) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-cache\r\nConnection: close\r\n",
        body.len()
    );
    for header in extra {
        head.push_str(header);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes())?;
    stream.write_all(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_reads_path_and_encoding() {
        let (path, br) =
            parse("GET /a%20b.png?x=1 HTTP/1.1\r\nHost: h\r\nAccept-Encoding: gzip, br\r\n\r\n")
                .unwrap();
        assert_eq!(path, "/a b.png");
        assert!(br);
        let (_, br) = parse("GET / HTTP/1.1\r\n\r\n").unwrap();
        assert!(!br);
        assert!(parse("POST / HTTP/1.1\r\n\r\n").is_none());
    }

    #[test]
    fn resolve_stays_inside_the_folder() {
        let dir = std::env::temp_dir().join(format!("inset-serve-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "hi").unwrap();
        assert_eq!(resolve(&dir, "/"), Some(dir.join("index.html")));
        assert_eq!(resolve(&dir, "/../index.html"), None);
        assert_eq!(resolve(&dir, "/missing"), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
