//! Why a round of sync stopped, as something a person can act on.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncError {
    /// devpit.app no longer knows this machine's session.
    SignedOut,
    /// devpit.app did not answer.
    Unreachable(String),
    /// devpit.app answered with a refusal, in its words.
    Refused(String),
    /// More than a blob or the account may hold.
    TooLarge(String),
    /// Another machine wrote first; the round reads again.
    Stale,
    /// The account's data is sealed with a key this machine does not hold.
    OtherKey,
    /// The blobs this machine synced are gone: sync was started over with a
    /// new key elsewhere, or devpit.app lost them.
    Reset,
    /// devpit.app served an older copy than this machine has already seen.
    Rollback,
    /// A blob that does not open, or opens to words devpit cannot read.
    Unreadable(String),
    /// This machine's own store or files.
    Local(String),
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::SignedOut => f.write_str(
                "devpit.app signed this computer out — sign in again in Settings → Account",
            ),
            SyncError::Unreachable(why) => write!(f, "devpit.app did not answer ({why})"),
            SyncError::Refused(why) => write!(f, "devpit.app refused: {why}"),
            SyncError::TooLarge(why) => f.write_str(why),
            SyncError::Stale => f.write_str("another machine synced at the same moment"),
            SyncError::OtherKey => f.write_str(
                "this account syncs with another key: pair this computer from one that syncs, or enter the recovery code",
            ),
            SyncError::Reset => f.write_str(
                "sync was started over on another machine: pair this computer again, or enter the new recovery code",
            ),
            SyncError::Rollback => f.write_str(
                "devpit.app answered with an older copy than this computer has seen, so nothing was taken from it",
            ),
            SyncError::Unreadable(why) | SyncError::Local(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for SyncError {}
