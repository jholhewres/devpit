//! Firefox, which keeps its cookies in the clear.
//!
//! `cookies.sqlite`, table `moz_cookies`, values as text. There is no
//! decryption step and no key: Firefox relies on the file's permissions, the
//! same thing Chromium falls back to when no keyring answered.
//!
//! The only real difference from [`crate::chromium`] is the epoch. Firefox
//! stores `expiry` as **seconds** since 1970, where Chromium stores
//! microseconds since 1601 — a store read with the wrong one gives every
//! cookie an expiry in the wrong millennium, which reads as "the import did
//! nothing" because the webview drops them all.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags};

use crate::{Cookie, Refused, Secret};

/// Every cookie in a Firefox store, opened read-only.
pub fn read(store: &Path) -> Result<Vec<Cookie>, Refused> {
    if !store.is_file() {
        return Err(Refused::NoStore(store.to_path_buf()));
    }
    let db = Connection::open_with_flags(
        store,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|err| {
        let said = err.to_string();
        if said.contains("locked") || said.contains("busy") {
            Refused::InUse(store.to_path_buf())
        } else {
            Refused::Unreadable {
                store: store.to_path_buf(),
                why: said,
            }
        }
    })?;

    let unreadable = |err: rusqlite::Error| Refused::Unreadable {
        store: store.to_path_buf(),
        why: err.to_string(),
    };

    let mut asked = db
        .prepare("SELECT host, name, value, path, expiry, isSecure, isHttpOnly FROM moz_cookies")
        .map_err(unreadable)?;
    let rows = asked
        .query_map([], |row| {
            Ok(Cookie {
                host: row.get(0)?,
                name: row.get(1)?,
                value: Secret::new(row.get::<_, String>(2)?),
                path: row.get(3)?,
                /* Seconds already, and zero means the session. */
                expires: match row.get::<_, i64>(4)? {
                    0 => None,
                    when => Some(when),
                },
                secure: row.get::<_, i64>(5)? != 0,
                http_only: row.get::<_, i64>(6)? != 0,
            })
        })
        .map_err(unreadable)?;

    rows.collect::<Result<Vec<_>, _>>().map_err(unreadable)
}

/// Every Firefox profile on this machine that has a cookie store.
///
/// A profile that has never been opened has no `cookies.sqlite`, and it is
/// left out rather than offered as an import that would find nothing.
pub fn stores_in(home: &Path) -> Vec<(String, PathBuf)> {
    let root = home.join(".mozilla/firefox");
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let store = entry.path().join("cookies.sqlite");
            store.is_file().then(|| {
                (
                    format!("Firefox · {}", entry.file_name().to_string_lossy()),
                    store,
                )
            })
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[cfg(test)]
#[path = "firefox_tests.rs"]
mod tests;
