//! Reading the cookie stores the browsers on this machine already have.
//!
//! A browser pane that opens every page at its login screen is a browser pane
//! nobody uses twice. This crate is what stops that: it reads what Chrome,
//! Firefox and Safari have already stored, so a page that is signed in stays
//! signed in.
//!
//! **Four rules, and they are the reason this is a crate and not a function.**
//!
//! 1. **A cookie value is a credential.** Nothing here writes one to a log,
//!    puts one in an error message, or returns one in a type whose `Debug`
//!    prints it — see [`Secret`]. What comes out of here goes into a webview
//!    session and nowhere else.
//! 2. **Another program's profile is read, never written.** Every store is
//!    opened read-only. A browser that is running has its SQLite locked, and
//!    the right answer to that is to say so, not to copy the file and hope.
//! 3. **Nothing is taken without being asked.** This crate reads what it is
//!    pointed at; deciding what to point it at is the caller's, and the caller
//!    asks a person first.
//! 4. **Half an import is not an import.** A store that cannot be decrypted,
//!    a keyring that refuses, a format that has moved — each one stops and
//!    says which, rather than returning the cookies it happened to manage.

pub mod chromium;
pub mod firefox;
pub mod keys;
pub mod safari;

use std::fmt;
use std::path::PathBuf;

/// A cookie value, which is a credential and is kept out of every log.
///
/// The whole reason this wrapper exists: `#[derive(Debug)]` on a struct with a
/// plain `String` in it is one `dbg!`, one `tracing::debug` or one
/// `format!("{err:?}")` away from a session token in a file. The value comes
/// out only through [`Secret::seen`], which a caller has to write on purpose.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// The value. Named so that reading the call site tells you a secret just
    /// left the box.
    pub fn seen(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        /* The length and nothing else: enough to tell "empty" from "there",
        which is the only thing a log has any business knowing. */
        write!(f, "Secret({} bytes)", self.0.len())
    }
}

/// One cookie, in the shape a webview is told about.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cookie {
    /// The host it belongs to, as the browser stored it — a leading dot means
    /// the subdomains too, and that is preserved rather than normalised away.
    pub host: String,
    pub name: String,
    pub value: Secret,
    pub path: String,
    /// Seconds since the unix epoch, or `None` for a session cookie that dies
    /// with the browser.
    pub expires: Option<i64>,
    pub secure: bool,
    pub http_only: bool,
}

/// Why a store could not be read, in the words the screen will use.
///
/// Each variant is a different thing to do about it, which is why they are not
/// one string: "close the browser" and "this machine's keyring said no" send a
/// person to different places.
#[derive(Debug)]
pub enum Refused {
    /// There is no store where one was expected.
    NoStore(PathBuf),
    /// The file is there and something else holds it — usually the browser
    /// itself, which locks its SQLite while it runs.
    InUse(PathBuf),
    /// The store opened and its shape is not one this knows.
    Unreadable { store: PathBuf, why: String },
    /// The values are encrypted and the key could not be had.
    NoKey(String),
    /// The key was had and the values did not decrypt with it.
    WrongKey,
    /// The store holds values encrypted under the *other* password. On Linux
    /// `v10` means Chromium's own fallback and `v11` means a keyring secret,
    /// and a profile can hold both — that is what a machine looks like after
    /// a keyring appeared, or went away.
    OtherPassword,
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoStore(at) => write!(f, "there is no cookie store at {}", at.display()),
            Self::InUse(at) => write!(
                f,
                "{} is held by something else — close the browser and try again",
                at.display()
            ),
            Self::Unreadable { store, why } => {
                write!(f, "{} is not a store this can read: {why}", store.display())
            }
            Self::NoKey(why) => write!(f, "the key to these cookies could not be had: {why}"),
            Self::OtherPassword => write!(
                f,
                "some of these cookies were encrypted with a key this machine cannot reach \
                 right now — the profile was used both with and without a desktop keyring. \
                 Installing libsecret-tools lets devpit ask the keyring for the other one"
            ),
            Self::WrongKey => write!(
                f,
                "these cookies did not decrypt — the key belongs to another profile, \
                 or this machine's keyring has been reset"
            ),
        }
    }
}

impl std::error::Error for Refused {}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
