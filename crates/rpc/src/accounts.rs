//! The Claude Code accounts devpit starts sessions in, and how much of each
//! one's plan is left.
//!
//! An account is a name and the `CLAUDE_CONFIG_DIR` its sign-in lives in.
//! Nothing here carries a token: the quota is read with one, and only what
//! it answered crosses to the window.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::PlanWindow;

/// One account: what the person calls it, and its configuration directory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeAccount {
    pub name: String,
    pub config_dir: String,
}

/// The account a project's new sessions open in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAccount {
    pub project_id: String,
    pub account: String,
}

/// Response of `accounts.read`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Accounts {
    pub accounts: Vec<ClaudeAccount>,
    #[serde(default)]
    pub projects: Vec<ProjectAccount>,
}

/// Why an account's quota has a number or does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum QuotaState {
    Read,
    /// The endpoint failed, refused or is cooling down after a 429.
    Unavailable,
    /// The saved access token lapsed; devpit never renews it, a session does.
    Expired,
    /// No Claude sign-in in that directory.
    SignedOut,
}

/// One account's quota windows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountQuota {
    pub name: String,
    pub config_dir: String,
    pub state: QuotaState,
    pub windows: Vec<PlanWindow>,
    /// The 5-hour window is past the point where a new session should go
    /// to another account.
    pub nearly_out: bool,
    /// Unix seconds.
    pub read_at: f64,
}

/// Response of `accounts.quota`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AccountQuotas {
    pub accounts: Vec<AccountQuota>,
}
