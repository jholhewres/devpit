//! Safari, which uses a format of its own.
//!
//! `Cookies.binarycookies` is not SQLite and not text. It is pages of records
//! with offsets into themselves, big-endian in the header and little-endian in
//! the body — which is the kind of detail that makes a parser written from
//! memory produce plausible nonsense rather than an error.
//!
//! ```text
//! "cook"            magic
//! u32 be            how many pages
//! u32 be × n        how long each page is
//! page:
//!   u32 le          0x00000100, a tag
//!   u32 le          how many cookies
//!   u32 le × n      where each one starts, from the page's own start
//!   cookie:
//!     u32 le        how long this record is
//!     u32 le        unused
//!     u32 le        flags: 1 secure, 4 http-only
//!     u32 le        unused
//!     u32 le × 4    offsets to url, name, path, value — from the record start
//!     u64 le        unused
//!     f64 le        expiry, seconds since 2001-01-01
//!     f64 le        creation, same epoch
//!     …             the four strings, each ending at a zero byte
//! ```
//!
//! **This machine cannot prove this reader.** Safari is macOS only, so what is
//! below is tested against a fixture this crate builds itself. That it reads a
//! real Safari store is something only a Mac can say, and until one does it is
//! written down as untested rather than assumed — the same way plan 23's macOS
//! stories waited for CI rather than claiming a machine that was not there.

use std::path::{Path, PathBuf};

use crate::{Cookie, Refused, Secret};

/// Safari counts from 2001-01-01, and the rest of the world from 1970-01-01.
const TO_UNIX: f64 = 978_307_200.0;

/// Reads four bytes as a big-endian number, or gives up.
fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le32(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn le64f(bytes: &[u8], at: usize) -> Option<f64> {
    Some(f64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

/// A NUL-terminated string starting at an offset.
fn text(bytes: &[u8], at: usize) -> Option<String> {
    let rest = bytes.get(at..)?;
    let end = rest
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(rest.len());
    Some(String::from_utf8_lossy(&rest[..end]).into_owned())
}

/// Every cookie in a `Cookies.binarycookies` file.
pub fn parse(bytes: &[u8]) -> Result<Vec<Cookie>, String> {
    if bytes.len() < 8 || &bytes[..4] != b"cook" {
        return Err("this is not a binarycookies file".to_owned());
    }
    let pages = be32(bytes, 4).ok_or("the page count is missing")? as usize;
    let mut sizes = Vec::with_capacity(pages);
    for page in 0..pages {
        sizes.push(be32(bytes, 8 + page * 4).ok_or("a page size is missing")? as usize);
    }

    let mut at = 8 + pages * 4;
    let mut found = Vec::new();
    for size in sizes {
        let page = bytes
            .get(at..at + size)
            .ok_or("a page runs past the file")?;
        at += size;
        found.extend(in_page(page)?);
    }
    Ok(found)
}

fn in_page(page: &[u8]) -> Result<Vec<Cookie>, String> {
    let count = le32(page, 4).ok_or("a page has no cookie count")? as usize;
    /* No `with_capacity(count)`: the count comes out of the file, and a
    corrupt or hostile one of 0xFFFFFFFF asks for ~450 GB, which Rust does not
    refuse — it aborts the process. The page cannot hold more cookies than it
    has room for offsets, and that is a bound the file cannot lie about. */
    if count > page.len() / 4 {
        return Err("a page claims more cookies than it has room for".to_owned());
    }
    let mut found = Vec::new();
    for one in 0..count {
        let start = le32(page, 8 + one * 4).ok_or("a cookie offset is missing")? as usize;
        let record = page.get(start..).ok_or("a cookie starts past its page")?;
        /* Bounded by the length the record itself declares, which the parser
        used to ignore entirely: without it a bogus offset reads straight into
        the next cookie's bytes and returns them as this one's value. */
        let size = le32(record, 0).ok_or("a cookie has no length")? as usize;
        if size < RECORD_HEAD {
            return Err("a cookie is shorter than a cookie can be".to_owned());
        }
        let record = record.get(..size).ok_or("a cookie runs past its page")?;
        found.push(one_cookie(record)?);
    }
    Ok(found)
}

/// The fixed part of a record, before the four strings: up to and including
/// the creation time at +48. Named because the fixture used to build 48 and
/// the parser never noticed, since it reads nothing past the expiry.
const RECORD_HEAD: usize = 56;

fn one_cookie(record: &[u8]) -> Result<Cookie, String> {
    let flags = le32(record, 8).ok_or("a cookie has no flags")?;
    let url = le32(record, 16).ok_or("a cookie has no url offset")? as usize;
    let name = le32(record, 20).ok_or("a cookie has no name offset")? as usize;
    let path = le32(record, 24).ok_or("a cookie has no path offset")? as usize;
    let value = le32(record, 28).ok_or("a cookie has no value offset")? as usize;
    let expires = le64f(record, 40).ok_or("a cookie has no expiry")?;

    Ok(Cookie {
        host: text(record, url).ok_or("a cookie's host runs past its record")?,
        name: text(record, name).ok_or("a cookie's name runs past its record")?,
        value: Secret::new(text(record, value).ok_or("a cookie's value runs past its record")?),
        path: text(record, path).ok_or("a cookie's path runs past its record")?,
        expires: (expires > 0.0).then_some((expires + TO_UNIX) as i64),
        secure: flags & 1 != 0,
        http_only: flags & 4 != 0,
    })
}

/// Every cookie in a Safari store.
pub fn read(store: &Path) -> Result<Vec<Cookie>, Refused> {
    if !store.is_file() {
        return Err(Refused::NoStore(store.to_path_buf()));
    }
    let bytes = std::fs::read(store).map_err(|err| Refused::Unreadable {
        store: store.to_path_buf(),
        why: err.to_string(),
    })?;
    parse(&bytes).map_err(|why| Refused::Unreadable {
        store: store.to_path_buf(),
        why,
    })
}

/// Safari's store, where macOS keeps it. Empty everywhere else.
pub fn stores_in(home: &Path) -> Vec<(String, PathBuf)> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    let store = home.join("Library/Cookies/Cookies.binarycookies");
    if store.is_file() {
        return vec![("Safari".to_owned(), store)];
    }
    /* Sandboxed Safari keeps it somewhere else, and has since Mojave. */
    let contained =
        home.join("Library/Containers/com.apple.Safari/Data/Library/Cookies/Cookies.binarycookies");
    if contained.is_file() {
        return vec![("Safari".to_owned(), contained)];
    }
    Vec::new()
}

#[cfg(test)]
#[path = "safari_tests.rs"]
mod tests;
