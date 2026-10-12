//! A namespace: one kind of thing that syncs — preferences, project
//! settings, the machines — read from this machine and written back to it.
//! Each is one blob on devpit.app, and the round treats them all alike, so a
//! new kind (the board, a project's memory) is a namespace more.

use std::collections::BTreeMap;

/// A field as this machine holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    /// `None` is a field this machine cleared.
    pub value: Option<String>,
    /// When it was set here, in milliseconds, when the store says. A value
    /// changed while offline is ordered by when it changed, not by when it
    /// was sent.
    pub at_ms: Option<u64>,
}

impl Seen {
    pub fn of(value: Option<String>, at_ms: Option<u64>) -> Seen {
        Seen { value, at_ms }
    }
}

/// Every record this machine has of a namespace, field by field. A record
/// that is not here — a project not cloned on this machine — is absent, and
/// absent says nothing: it is never taken for a removal.
pub type Here = BTreeMap<String, BTreeMap<String, Seen>>;

pub trait Namespace {
    /// What its blob's name is drawn from. Never shown to devpit.app.
    fn name(&self) -> &'static str;

    /// What this machine has now, of what syncs. Anything outside the
    /// namespace's allowlist is never read.
    fn here(&self) -> Result<Here, String>;

    /// Writes what the merge settled for a field of a record this machine
    /// has. A field outside the allowlist, or a value that does not fit, is
    /// left alone: whatever arrives is checked again here.
    fn write(&self, record: &str, field: &str, value: Option<&str>) -> Result<(), String>;
}
