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

/// One POST: what it carried, and which pane it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posted {
    pub body: String,
    /// The pane named in the query, when the hook was fired inside one of our
    /// terminals. Absent for a headless turn, which belongs to a card rather
    /// than to a pane.
    pub pane: Option<String>,
}

/// The body of a POST, or nothing.
///
/// Written by hand rather than with an HTTP crate: one route, one method, one
/// caller on loopback. A dependency here would be several thousand lines to
/// parse a request this already refuses to over-read.
pub fn read_request(stream: &mut TcpStream) -> Option<Posted> {
    read_post(BufReader::new(stream.try_clone().ok()?))
}

/// The same, from anything that yields bytes.
///
/// Split off the socket so a test can hand it a request instead of opening a
/// port. The ceiling below is the only thing between a `content-length`
/// somebody else wrote and an allocation of exactly that size, and a rule a
/// test cannot call is a rule the test cannot guard.
pub(crate) fn read_post(mut reader: impl BufRead) -> Option<Posted> {
    let mut length = 0usize;
    let mut pane = None;
    let mut first = true;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if first {
            // `POST /hook?pane=leaf_01... HTTP/1.1`. The only line that is
            // not a header, and the only place the pane could travel without
            // touching the JSON the agent itself wrote.
            pane = pane_in(line);
            first = false;
            continue;
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
    Some(Posted {
        body: String::from_utf8(body).ok()?,
        pane,
    })
}

/// The pane named in a request line, if one is.
///
/// Refused unless it looks like one of ours. This arrives from a shell we
/// wrote but through a process we did not, and it goes on to key a map and
/// reach the screen — a value nobody checked is a value somebody else can
/// choose.
fn pane_in(request_line: &str) -> Option<String> {
    let target = request_line.split_whitespace().nth(1)?;
    let query = target.split_once('?')?.1;
    let value = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("pane="))?;
    let named = value.trim();
    let ours = !named.is_empty()
        && named.len() <= 64
        && named
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    ours.then(|| named.to_owned())
}

#[cfg(test)]
#[path = "post_tests.rs"]
mod tests;
