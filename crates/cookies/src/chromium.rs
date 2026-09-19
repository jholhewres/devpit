//! Chrome, Edge, Brave, Chromium, Vivaldi — one format, several names.
//!
//! The store is SQLite. The values in it are not: since Chrome 33 they live in
//! `encrypted_value` with a three-byte version prefix saying how, and `value`
//! is left empty. Reading the table is the easy half.
//!
//! **On Linux** the scheme is AES-128-CBC, and the two prefixes differ only in
//! where the password comes from:
//!
//! - `v10` — the password is the literal string `peanuts`. That is not a
//!   secret and never was: it is what Chromium uses when no keyring answered,
//!   and it means those cookies are protected by file permissions alone.
//! - `v11` — the password is a secret the desktop keyring holds.
//!
//! Either way the key is PBKDF2-HMAC-SHA1 over that password with the salt
//! `saltysalt`, and the initialisation vector is sixteen spaces. The iteration
//! count is **1** on Linux and **1003** on macOS, which is the only part of
//! this that differs between them.
//!
//! None of that is a cipher this crate implements. The primitives are the
//! RustCrypto ones; what is here is the shape of the file and the shape of the
//! key.

use std::path::{Path, PathBuf};

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use rusqlite::{Connection, OpenFlags};

use sha2::{Digest, Sha256};

use crate::{Cookie, Refused, Secret};

type Decryptor = cbc::Decryptor<aes::Aes128>;

/// The salt Chromium has used since the scheme existed.
const SALT: &[u8] = b"saltysalt";
/// Sixteen spaces. Not a nonce, not random, and not ours to change.
const IV: [u8; 16] = [b' '; 16];
/// The password Chromium falls back to when no keyring answered.
const NO_KEYRING: &str = "peanuts";

/// Where a decryption key comes from, which is the only thing that differs
/// between a machine with a keyring and one without.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Password {
    /// The keyring answered, and this is what it said.
    Keyring(String),
    /// It did not, and Chromium's own fallback is what encrypted these.
    Fallback,
}

impl Password {
    /// The prefix Chromium wrote on values it encrypted with this password.
    ///
    /// On Linux the three bytes name where the password came from, not the
    /// cipher: `v10` is the `peanuts` fallback and `v11` is a keyring secret.
    /// Reading one with the other's key produces `WrongKey`, which reads as a
    /// corrupt profile and is not.
    pub fn prefix(&self) -> [u8; 3] {
        match self {
            Self::Keyring(_) => *b"v11",
            Self::Fallback => *b"v10",
        }
    }

    fn said(&self) -> &str {
        match self {
            Self::Keyring(secret) => secret,
            Self::Fallback => NO_KEYRING,
        }
    }
}

/// How many times to stretch the password, which is the one number that
/// differs by platform.
const fn rounds() -> u32 {
    if cfg!(target_os = "macos") {
        1003
    } else {
        1
    }
}

/// The AES key a password produces.
///
/// Its own function because it is the part worth a test: a wrong iteration
/// count or a wrong salt produces a key that decrypts nothing, and the failure
/// looks exactly like a wrong password.
pub fn key_from(password: &Password) -> [u8; 16] {
    let mut key = [0_u8; 16];
    /* Cannot fail for a 16-byte output. */
    let _ = pbkdf2::pbkdf2::<hmac::Hmac<sha1::Sha1>>(
        password.said().as_bytes(),
        SALT,
        rounds(),
        &mut key,
    );
    key
}

/// Chromium counts microseconds since 1601-01-01, and the rest of the world
/// counts seconds since 1970-01-01.
const TO_UNIX: i64 = 11_644_473_600;

/// A stored timestamp as seconds since the unix epoch, or `None` when the
/// cookie has no expiry and dies with the session.
pub fn expiry(stored: i64) -> Option<i64> {
    if stored == 0 {
        return None;
    }
    Some(stored / 1_000_000 - TO_UNIX)
}

/// Turns one stored value into the text a cookie carries.
///
/// A value with no prefix is one Chromium never encrypted, which is what an
/// old profile and a machine with encryption turned off both look like.
pub fn plain(
    stored: &[u8],
    key: &[u8; 16],
    host: &str,
    password: &Password,
) -> Result<Secret, Refused> {
    if stored.is_empty() {
        return Ok(Secret::new(""));
    }
    let prefix = password.prefix();
    let Some(body) = stored.strip_prefix(&prefix) else {
        if stored.starts_with(b"v10") || stored.starts_with(b"v11") {
            /* The other prefix. On Linux these name the *password*, not the
            algorithm — `v10` is Chromium's `peanuts` fallback and `v11` is a
            keyring secret — so the caller's key cannot open this one, and
            saying `WrongKey` would blame the profile for the reader's
            mistake. A store holding both is ordinary: it is what a machine
            looks like after a keyring appeared, or went away. */
            return Err(Refused::OtherPassword);
        }
        /* Not encrypted. Stored as bytes, read as text. */
        return Ok(Secret::new(String::from_utf8_lossy(stored).into_owned()));
    };

    let mut buffer = body.to_vec();
    let opened = Decryptor::new(key.into(), &IV.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .map_err(|_| Refused::WrongKey)?;

    Ok(Secret::new(
        String::from_utf8_lossy(without_domain_hash(opened, host)).into_owned(),
    ))
}

/// Chromium 130 and later put the SHA-256 of the cookie's own host in front
/// of the plaintext. Older ones do not, and the version is not in the file.
///
/// **Told apart by computing that hash, not by guessing.** The first version
/// of this asked whether the leading 32 bytes looked printable, and that was
/// wrong in both directions — proved in review by running it:
///
/// - a value carrying any byte outside printable ASCII had 32 bytes eaten.
///   `{"user":"José Pérez",…}` came back as `2837"}`. Silent: the page simply
///   is not signed in and nothing says why;
/// - and at the boundary, an empty value under Chromium 130 decrypts to
///   exactly 32 bytes, the length check returned early, and the whole hash
///   came back *as* the cookie.
///
/// The host is in the same row the value came from. There was never anything
/// to guess.
fn without_domain_hash<'a>(opened: &'a [u8], host: &str) -> &'a [u8] {
    if opened.len() < 32 {
        return opened;
    }
    let (front, rest) = opened.split_at(32);
    let expected = Sha256::digest(host.as_bytes());
    if front == expected.as_slice() {
        return rest;
    }
    opened
}

/// Every cookie in a Chromium store.
///
/// Opened read-only, and the file is never written: this is somebody else's
/// browser profile. A store the browser has open answers `InUse` rather than
/// being copied somewhere and read behind its back.
pub fn read(store: &Path, password: &Password) -> Result<Vec<Cookie>, Refused> {
    if !store.is_file() {
        return Err(Refused::NoStore(store.to_path_buf()));
    }
    let open = Connection::open_with_flags(
        store,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    );
    let db = open.map_err(|err| held(store, err))?;
    let key = key_from(password);

    let mut asked = db
        .prepare(
            "SELECT host_key, name, value, encrypted_value, path, expires_utc, \
             is_secure, is_httponly FROM cookies",
        )
        .map_err(|err| Refused::Unreadable {
            store: store.to_path_buf(),
            why: err.to_string(),
        })?;

    let rows = asked
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Vec<u8>>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })
        .map_err(|err| Refused::Unreadable {
            store: store.to_path_buf(),
            why: err.to_string(),
        })?;

    let mut found = Vec::new();
    for row in rows {
        let (host, name, value, encrypted, path, expires, secure, http_only) =
            row.map_err(|err| Refused::Unreadable {
                store: store.to_path_buf(),
                why: err.to_string(),
            })?;
        /* Half an import is not an import: one value that will not decrypt
        stops the whole store rather than leaving a person signed into some of
        their sites and wondering which. */
        let value = if encrypted.is_empty() {
            Secret::new(value)
        } else {
            plain(&encrypted, &key, &host, password)?
        };
        found.push(Cookie {
            host,
            name,
            value,
            path,
            expires: expiry(expires),
            secure: secure != 0,
            http_only: http_only != 0,
        });
    }
    Ok(found)
}

/// A store SQLite would not open, told apart by why.
fn held(store: &Path, err: rusqlite::Error) -> Refused {
    let said = err.to_string();
    if said.contains("locked") || said.contains("busy") {
        return Refused::InUse(store.to_path_buf());
    }
    Refused::Unreadable {
        store: store.to_path_buf(),
        why: said,
    }
}

/// The profile directories of every Chromium-family browser on this machine.
///
/// A list of where to look, not a promise that anything is there. The caller
/// asks a person which of these to read — nothing is imported because it was
/// found.
pub fn stores_in(home: &Path) -> Vec<(String, PathBuf)> {
    let families = [
        ("Google Chrome", ".config/google-chrome"),
        ("Chromium", ".config/chromium"),
        ("Microsoft Edge", ".config/microsoft-edge"),
        ("Brave", ".config/BraveSoftware/Brave-Browser"),
        ("Vivaldi", ".config/vivaldi"),
    ];
    let mut found = Vec::new();
    for (name, under) in families {
        let root = home.join(under);
        if !root.is_dir() {
            continue;
        }
        /* `Default`, and `Profile 1`, `Profile 2`… — a person with two Google
        accounts has two, and importing from the wrong one looks like the
        import silently not working. */
        let Ok(entries) = std::fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let store = entry.path().join("Cookies");
            if store.is_file() {
                let profile = entry.file_name().to_string_lossy().into_owned();
                found.push((format!("{name} · {profile}"), store));
            }
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

#[cfg(test)]
#[path = "chromium_tests.rs"]
mod tests;
