//! Sync between the person's machines, as Settings → Account shows it.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Where sync stands on this machine: the answer of every sync command.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    /// Signed in to devpit.app here, which sync needs.
    pub signed_in: bool,
    /// Sync is on on this machine. Off unless turned on.
    pub on: bool,
    /// This machine holds the account's key. On without it, it waits to be
    /// joined: from another machine's offer, or with the recovery code.
    pub has_key: bool,
    /// When the last round finished, in seconds since the epoch.
    pub synced_at: Option<f64>,
    /// Why the last round stopped, while it has not gone through since.
    pub problem: Option<String>,
    /// The account's machines that sync, this one among them.
    pub machines: Vec<SyncMachine>,
    /// The offer this machine is making, while it lasts.
    pub offering: Option<SyncOffer>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncMachine {
    pub id: String,
    pub name: String,
    /// When it last synced, to the hour, in seconds since the epoch.
    pub seen_at: Option<f64>,
    pub this_one: bool,
}

/// What the person carries to another machine to give it the key: good once,
/// for two minutes, through devpit.app's relay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncOffer {
    pub text: String,
    pub expires_at: f64,
}

/// The key as the person writes it down. Shown only when asked for.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SyncRecovery {
    pub code: String,
}
