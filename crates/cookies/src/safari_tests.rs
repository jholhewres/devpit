//! Safari's own format, against a fixture this builds.
//!
//! No Mac here, so this proves the parser and not that a real Safari store
//! reads. That distinction is written down rather than blurred.

use super::*;

/// Builds one page holding one cookie, in the layout the module documents.
fn a_file(host: &str, name: &str, path: &str, value: &str, expires: f64, flags: u32) -> Vec<u8> {
    /* The record: a fixed head of 48 bytes, then the four strings. */
    let head = 48_usize;
    let at_url = head;
    let at_name = at_url + host.len() + 1;
    let at_path = at_name + name.len() + 1;
    let at_value = at_path + path.len() + 1;
    let size = at_value + value.len() + 1;

    let mut record = vec![0_u8; size];
    record[0..4].copy_from_slice(&(size as u32).to_le_bytes());
    record[8..12].copy_from_slice(&flags.to_le_bytes());
    record[16..20].copy_from_slice(&(at_url as u32).to_le_bytes());
    record[20..24].copy_from_slice(&(at_name as u32).to_le_bytes());
    record[24..28].copy_from_slice(&(at_path as u32).to_le_bytes());
    record[28..32].copy_from_slice(&(at_value as u32).to_le_bytes());
    record[40..48].copy_from_slice(&expires.to_le_bytes());
    record[at_url..at_url + host.len()].copy_from_slice(host.as_bytes());
    record[at_name..at_name + name.len()].copy_from_slice(name.as_bytes());
    record[at_path..at_path + path.len()].copy_from_slice(path.as_bytes());
    record[at_value..at_value + value.len()].copy_from_slice(value.as_bytes());

    /* The page: tag, count, one offset, then the record. */
    let mut page = Vec::new();
    page.extend_from_slice(&0x0000_0100_u32.to_le_bytes());
    page.extend_from_slice(&1_u32.to_le_bytes());
    page.extend_from_slice(&12_u32.to_le_bytes());
    page.extend_from_slice(&record);

    /* The file: magic, page count, page size, then the page. Big-endian here
    and little-endian in the body, which is the trap this fixture pins. */
    let mut file = Vec::new();
    file.extend_from_slice(b"cook");
    file.extend_from_slice(&1_u32.to_be_bytes());
    file.extend_from_slice(&(page.len() as u32).to_be_bytes());
    file.extend_from_slice(&page);
    file
}

#[test]
fn a_cookie_is_read_out_of_the_pages_and_offsets() {
    /* 2024-01-01T00:00:00Z is 1704067200 unix. Safari counts from 2001-01-01,
    so the same instant is 1704067200 - 978307200 = 725760000 there. */
    let bytes = a_file(
        ".example.com",
        "session",
        "/",
        "a-value",
        725_760_000.0,
        1 | 4,
    );
    let found = parse(&bytes).expect("the fixture parses");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].host, ".example.com");
    assert_eq!(found[0].name, "session");
    assert_eq!(found[0].path, "/");
    assert_eq!(found[0].value.seen(), "a-value");
    assert_eq!(
        found[0].expires,
        Some(1_704_067_200),
        "the 2001 epoch moved"
    );
    assert!(found[0].secure, "flag 1");
    assert!(found[0].http_only, "flag 4");
}

#[test]
fn the_flags_are_read_apart_from_each_other() {
    let neither = parse(&a_file("a.test", "n", "/", "v", 0.0, 0)).unwrap();
    assert!(!neither[0].secure && !neither[0].http_only);
    let secure = parse(&a_file("a.test", "n", "/", "v", 0.0, 1)).unwrap();
    assert!(secure[0].secure && !secure[0].http_only);
    let only = parse(&a_file("a.test", "n", "/", "v", 0.0, 4)).unwrap();
    assert!(!only[0].secure && only[0].http_only);
}

#[test]
fn a_cookie_with_no_expiry_dies_with_the_session() {
    let found = parse(&a_file("a.test", "n", "/", "v", 0.0, 0)).unwrap();
    assert_eq!(found[0].expires, None);
}

/// Something that is not this format says so rather than producing plausible
/// nonsense, which is what a parser written from memory does.
#[test]
fn something_that_is_not_a_binarycookies_file_is_refused() {
    assert!(parse(b"").is_err());
    assert!(parse(b"not a cookie file").is_err());
    assert!(parse(b"cook").is_err(), "the magic alone is not a file");
    /* A truncated page is refused rather than read past. */
    let mut short = a_file("a.test", "n", "/", "v", 0.0, 0);
    short.truncate(short.len() - 10);
    assert!(parse(&short).is_err());
}

/// Safari is macOS-only, and this machine is not one.
#[test]
fn nothing_is_offered_where_safari_cannot_be() {
    if !cfg!(target_os = "macos") {
        let home = std::env::temp_dir();
        assert!(stores_in(&home).is_empty());
    }
}

/// A record that declares less than a header refuses, rather than being
/// stretched to fit. My first attempt at this bound wrote `size.max(HEAD)`,
/// which extends the slice for exactly the malformed input it was meant to
/// reject — and the fixture never exercised it, so the test stayed green.
#[test]
fn a_record_shorter_than_its_own_header_is_refused() {
    let mut bytes = a_file("a.test", "n", "/", "v", 0.0, 0);
    /* The record starts 12 bytes into the page, and the page 12 into the
    file: rewrite the length it declares to something impossible. */
    let at = 12 + 12;
    bytes[at..at + 4].copy_from_slice(&8_u32.to_le_bytes());
    let refused = parse(&bytes).expect_err("a record of 8 bytes cannot hold a header");
    assert!(refused.contains("shorter"), "{refused}");
}
