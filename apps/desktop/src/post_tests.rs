//! What the hook listener accepts, and what it refuses to read.
//!
//! Fed bytes rather than a socket. The rules worth pinning here are the ones
//! that decide how much memory a request gets to ask for, and those do not
//! need a port to exercise.

use crate::post::{read_post, MOST_BYTES};

/// A request, assembled the way `curl --data-binary` sends one.
fn post(body: &str) -> Vec<u8> {
    format!(
        "POST /hook HTTP/1.1\r\nhost: 127.0.0.1\r\ncontent-type: application/json\r\n\
         content-length: {}\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

#[test]
fn a_well_formed_post_gives_up_its_body() {
    let body = r#"{"hook_event_name":"Stop","session_id":"s1"}"#;
    let raw = post(body);
    assert_eq!(read_post(&raw[..]), Some(body.to_owned()));
}

/// A header can arrive in any case, and `Content-Length` is how most senders
/// spell it. Matching only the lowercase form would drop every one of them.
#[test]
fn the_length_header_is_read_whatever_its_case() {
    let raw = b"POST /hook HTTP/1.1\r\nContent-Length: 2\r\n\r\nhi";
    assert_eq!(read_post(&raw[..]), Some("hi".to_owned()));
}

/// The rule this file exists for.
///
/// The length is a number somebody else wrote, and the next line after the
/// check allocates exactly that many bytes. Without the ceiling, one request
/// claiming a gigabyte is one gigabyte asked for.
///
/// The body is sent in full, all of it. The first version of this test sent
/// the header alone, which passes either way: with the ceiling the length is
/// refused, and without it the short body fails the read. Two paths, one
/// answer, and a test that cannot tell them apart — the same mistake this
/// file was written to stop making.
#[test]
fn a_length_past_the_ceiling_is_refused_before_anything_is_allocated() {
    let body = "x".repeat(MOST_BYTES + 1);
    let raw = format!(
        "POST /hook HTTP/1.1\r\ncontent-length: {}\r\n\r\n{body}",
        MOST_BYTES + 1
    );
    assert_eq!(read_post(raw.as_bytes()), None);
}

#[test]
fn a_length_at_the_ceiling_is_still_read() {
    let body = "x".repeat(MOST_BYTES);
    let raw = format!("POST /hook HTTP/1.1\r\ncontent-length: {MOST_BYTES}\r\n\r\n{body}");
    assert_eq!(read_post(raw.as_bytes()), Some(body));
}

#[test]
fn a_request_with_no_body_is_nothing_to_read() {
    let raw = b"POST /hook HTTP/1.1\r\ncontent-length: 0\r\n\r\n";
    assert_eq!(read_post(&raw[..]), None);
}

/// A length that is not a number ends the read rather than defaulting to one.
/// Defaulting would turn a malformed header into a body of the wrong size.
#[test]
fn a_length_that_is_not_a_number_is_refused() {
    let raw = b"POST /hook HTTP/1.1\r\ncontent-length: lots\r\n\r\nhi";
    assert_eq!(read_post(&raw[..]), None);
}

/// Connection closed mid-headers. `read_line` returns zero and the loop has
/// to stop, not spin.
#[test]
fn a_truncated_request_ends_rather_than_spins() {
    let raw = b"POST /hook HTTP/1.1\r\ncontent-length: 5\r\n";
    assert_eq!(read_post(&raw[..]), None);
}

/// The body arrives shorter than the header promised. Reading it as far as it
/// goes would hand half a payload to the parser.
#[test]
fn a_body_shorter_than_promised_is_refused() {
    let raw = b"POST /hook HTTP/1.1\r\ncontent-length: 40\r\n\r\nshort";
    assert_eq!(read_post(&raw[..]), None);
}
