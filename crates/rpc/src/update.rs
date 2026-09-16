//! What the app knows about an update, as the window sees it.
//!
//! One state at a time, and the window draws whatever it is handed: the rules
//! for moving between them live in the app (`update::next`), because a state
//! machine split across a process boundary is a state machine with two
//! opinions.

use serde::{Deserialize, Serialize};
use specta::Type;

/// How this copy of devpit was installed, which decides what an update may do
/// to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum InstallKind {
    /// A single file the app can replace by itself.
    AppImage,
    /// A system package: downloaded and verified here, installed by the person
    /// with the command we show. The app never runs a package manager.
    Deb,
    /// A package the machine's own tooling looks after — a repackage, a Nix
    /// profile, a container. Nothing is downloaded.
    ExternallyManaged,
    /// `make dev`, a `cargo run`, or anything else with no bundle around it.
    /// Nothing is installed from here.
    Unmanaged,
}

/// Where the update is, and what may be done about it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum UpdateStatus {
    /// Nothing known, or nothing newer.
    Idle,
    Checking,
    Available {
        version: String,
        notes: String,
        kind: InstallKind,
        /// True when the answer came from a test feed rather than the real
        /// one. The window says so: an offer nobody can install has to look
        /// different from one they can.
        ///
        /// Renamed by hand: `rename_all` on a tagged enum renames the
        /// variants, not the fields inside them.
        #[serde(rename = "testFeed")]
        test_feed: bool,
    },
    Downloading {
        percent: u8,
    },
    /// Downloaded and verified. From here on the app refuses to start new work.
    Ready {
        version: String,
    },
    /// Waiting for work that is already running, with what it is waiting on.
    Waiting {
        runs: u32,
        turns: u32,
        /// Seconds since the epoch, so the window can say how long.
        since: f64,
    },
    /// Past the point of return: the installer is running.
    Installing,
    /// A package the person installs, with the command to do it.
    ManualInstall {
        command: String,
        path: String,
    },
    ExternallyManaged,
    Failed {
        message: String,
        /// False once the install has committed: there is nothing to retry.
        recoverable: bool,
    },
}
