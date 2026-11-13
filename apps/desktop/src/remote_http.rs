//! The requests the remote host reads, by hand like the hook listener's:
//! a method, a path, a few headers and a small body, every length checked
//! before anything is allocated.

use std::collections::HashMap;
use std::io::{BufRead, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MOST_LINE: usize = 8 * 1024;
const MOST_HEADERS: usize = 64;
/// A pairing is the only body, and it is a few hundred bytes.
const MOST_BODY: usize = 16 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    pub method: String,
    /// Without the query.
    pub path: String,
    /// After the `?`, read from the whole request line.
    pub query: String,
    /// Names lowercased.
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

pub(crate) fn read(reader: impl BufRead) -> Option<Request> {
    read_up_to(reader, MOST_BODY)
}

/// [`read`], with a body up to `most_body` bytes.
pub(crate) fn read_up_to(mut reader: impl BufRead, most_body: usize) -> Option<Request> {
    let mut request = read_head(&mut reader)?;
    read_body(&mut reader, &mut request, most_body)?;
    Some(request)
}

/// The request line and headers, the body left unread.
pub(crate) fn read_head(mut reader: impl BufRead) -> Option<Request> {
    let mut first = String::new();
    line(&mut reader, &mut first)?;
    let mut words = first.split_whitespace();
    let method = words.next()?.to_owned();
    let target = words.next()?;
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let (path, query) = (path.to_owned(), query.to_owned());
    let mut headers = HashMap::new();
    loop {
        if headers.len() > MOST_HEADERS {
            return None;
        }
        let mut header = String::new();
        line(&mut reader, &mut header)?;
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        let (name, value) = header.split_once(':')?;
        headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_owned());
    }
    Some(Request {
        method,
        path,
        query,
        headers,
        body: Vec::new(),
    })
}

/// The body `content-length` gives, up to `most` bytes, kept as it arrives
/// rather than allocated from what the header claims.
pub(crate) fn read_body(
    mut reader: impl BufRead,
    request: &mut Request,
    most: usize,
) -> Option<()> {
    let length: usize = match request.header("content-length") {
        Some(length) => length.parse().ok()?,
        None => 0,
    };
    if length > most {
        return None;
    }
    let mut body = Vec::new();
    Read::take(reader.by_ref(), length as u64)
        .read_to_end(&mut body)
        .ok()?;
    if body.len() != length {
        return None;
    }
    request.body = body;
    Some(())
}

/// A socket read that gives up at `until`: a read timeout alone starts over
/// with every byte, so a head sent a byte at a time would hold a thread for good.
pub(crate) struct Deadline {
    stream: TcpStream,
    until: Instant,
}

impl Deadline {
    pub(crate) fn new(stream: TcpStream, within: Duration) -> Deadline {
        Deadline {
            stream,
            until: Instant::now() + within,
        }
    }

    /// Moves the deadline to `within` from now.
    pub(crate) fn renew(&mut self, within: Duration) {
        self.until = Instant::now() + within;
    }
}

impl Read for Deadline {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let left = self.until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(std::io::ErrorKind::TimedOut.into());
        }
        self.stream.set_read_timeout(Some(left))?;
        self.stream.read(buf)
    }
}

/// A connection served; one fewer open once it is dropped.
pub(crate) struct Opened(Arc<AtomicUsize>);

impl Drop for Opened {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// One more connection, unless `most` are open already.
pub(crate) fn open_one(open: &Arc<AtomicUsize>, most: usize) -> Option<Opened> {
    open.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |now| {
        (now < most).then_some(now + 1)
    })
    .ok()?;
    Some(Opened(Arc::clone(open)))
}

fn line(reader: &mut impl BufRead, into: &mut String) -> Option<()> {
    let read = Read::take(reader.by_ref(), MOST_LINE as u64)
        .read_line(into)
        .ok()?;
    (read > 0 && into.ends_with('\n')).then_some(())
}

/// A whole reply, closed after.
pub(crate) fn reply(stream: &mut impl Write, status: &str, kind: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {status}\r\ncontent-type: {kind}\r\ncontent-length: {}\r\n\
         cache-control: no-store\r\nx-content-type-options: nosniff\r\n\
         referrer-policy: no-referrer\r\nx-frame-options: DENY\r\n\
         content-security-policy: default-src 'self'; connect-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; frame-ancestors 'none'\r\n\
         connection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

/// The media type a viewer file is served as, by its extension.
pub(crate) fn kind_of(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("woff2") => "font/woff2",
        Some("json" | "webmanifest") => "application/json",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::{kind_of, read};

    #[test]
    fn a_request_is_read_with_its_headers_and_body() {
        let raw = b"POST /api/pair?x=1 HTTP/1.1\r\nHost: m.ts.net\r\nTailscale-User-Login: jo@x.com\r\nContent-Length: 4\r\n\r\nbody";
        let request = read(&raw[..]).expect("read");
        assert_eq!(
            (request.method.as_str(), request.path.as_str()),
            ("POST", "/api/pair")
        );
        assert_eq!(request.header("tailscale-user-login"), Some("jo@x.com"));
        assert_eq!(request.body, b"body");
    }

    #[test]
    fn what_is_too_big_or_cut_short_is_refused() {
        let big = format!("POST / HTTP/1.1\r\ncontent-length: {}\r\n\r\n", 1024 * 1024);
        assert!(read(big.as_bytes()).is_none());
        assert!(read(&b"GET / HTTP/1.1\r\nhost: x"[..]).is_none());
        let long = format!("GET /{} HTTP/1.1\r\n\r\n", "a".repeat(20_000));
        assert!(read(long.as_bytes()).is_none());
    }

    #[test]
    fn files_are_served_as_what_they_are() {
        assert_eq!(kind_of("assets/remote-1a2b.js"), "text/javascript");
        assert_eq!(kind_of("remote.html"), "text/html; charset=utf-8");
        assert_eq!(kind_of("weird"), "application/octet-stream");
    }
}
