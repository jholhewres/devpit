//! Reading one POST off a socket, by hand.
//!
//! One route, one method, one caller on loopback. An HTTP crate here would be
//! several thousand lines to parse a request this already refuses to
//! over-read.

use std::io::{BufRead, BufReader};
use std::net::TcpStream;

/// A payload bigger than this is not one of ours.
///
/// A `Stop` carries the last thing the agent said, which can be long, so the
/// ceiling is generous — but a ceiling there is, because this reads from a
/// socket and an unbounded read is a way to spend all the memory on the box.
pub(crate) const MOST_BYTES: usize = 256 * 1024;

/// The body of a POST, or nothing.
///
/// Written by hand rather than with an HTTP crate: one route, one method, one
/// caller on loopback. A dependency here would be several thousand lines to
/// parse a request this already refuses to over-read.
pub fn read_request(stream: &mut TcpStream) -> Option<String> {
    read_post(BufReader::new(stream.try_clone().ok()?))
}

/// The same, from anything that yields bytes.
///
/// Split off the socket so a test can hand it a request instead of opening a
/// port. The ceiling below is the only thing between a `content-length`
/// somebody else wrote and an allocation of exactly that size, and a rule a
/// test cannot call is a rule the test cannot guard.
pub(crate) fn read_post(mut reader: impl BufRead) -> Option<String> {
    let mut length = 0usize;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
            length = value.trim().parse().ok()?;
        }
    }

    if length == 0 || length > MOST_BYTES {
        return None;
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;
    String::from_utf8(body).ok()
}

#[cfg(test)]
#[path = "post_tests.rs"]
mod tests;
