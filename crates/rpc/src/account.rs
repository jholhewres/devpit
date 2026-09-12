//! The account this install is signed in as.
//!
//! The app never sees a password. Signing in opens the browser at the accounts
//! site, the person approves a short code there, and the app collects a device
//! token it keeps on disk. Everything here describes that, and nothing here
//! carries a secret to the screen.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Who the person is, as the accounts server describes them.
///
/// Two fields today, because two are what the server has. The shape is an
/// object rather than a bare string so a handle, an avatar and a plan can
/// arrive later without every caller changing.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub email: String,
    /// What the person calls themselves, when they have set one. Absent rather
    /// than derived from the address: a name nobody chose is worse than none.
    pub name: Option<String>,
    /// RFC 3339, in UTC.
    pub created_at: String,
}

/// What the account pane draws.
///
/// `account` is null for "nobody is signed in", which is a state and not an
/// error — a screen that has to catch a failure to draw its signed-out half
/// draws it late.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Membership {
    pub account: Option<Account>,
    /// Where the browser is sent, so the screen can name it without hard-coding
    /// a domain the build might not be pointing at.
    pub origin: String,
    /// False when the token on disk was refused. The person is signed out, and
    /// telling them it expired is kinder than pretending they never signed in.
    pub expired: bool,
}

/// A sign-in that has started but not finished.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SignIn {
    /// Shown in the app, and already in the url the browser opened. Typing it
    /// is the fallback, not the path.
    pub user_code: String,
    pub verify_url: String,
    pub expires_in_seconds: u32,
    /// How often the app may ask. The server decides, so a polling loop cannot
    /// become a load test by being written badly here.
    pub interval_seconds: u32,
}

/// Where a sign-in stands, each time the screen asks.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum SignInState {
    /// Nobody has approved it yet. Ask again after `interval_seconds`.
    Waiting,
    /// Done. The token is already on disk; the screen gets the person.
    Signed { account: Account },
    /// The code ran out, or was already used. Start again.
    Expired,
}
