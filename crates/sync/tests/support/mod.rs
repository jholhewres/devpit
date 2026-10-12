//! Two machines of one account, each a devpit home of its own, and what
//! stands where devpit.app does: a stand-in that keeps blobs by its rules,
//! or devpit-app itself when a checkout and a Postgres are at hand.

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use devpit_core::Store;
use devpit_sync::machines::Machines;
use devpit_sync::prefs::Prefs;
use devpit_sync::projects::Projects;
use devpit_sync::{
    files, round, Blobs, Listed, Namespace, Outcome, State, SyncError, SyncKey, MOST_BLOB,
};

/// The account's room, as devpit-app gives it.
pub const MOST_BYTES: usize = 16 * 1024 * 1024;
pub const MOST_BLOBS: usize = 256;

/// devpit.app's sync, kept in memory by the same rules: a version named
/// on every write, a ceiling per blob and a room per account. Every blob
/// it was sent is kept, for a test to look at.
/// Blobs by name: each one's version and bytes.
pub type Kept = BTreeMap<String, (u64, Vec<u8>)>;

/// One blob as it was sent: its name, the version it became, its bytes.
pub type Sent = (String, u64, Vec<u8>);

#[derive(Clone, Default)]
pub struct Stand {
    blobs: Arc<Mutex<Kept>>,
    pub sent: Arc<Mutex<Vec<Sent>>>,
}

impl Stand {
    pub fn kept(&self) -> Kept {
        self.blobs.lock().unwrap().clone()
    }

    /// Replaces a blob behind the machines' backs, as a server could.
    pub fn tamper(&self, name: &str, version: u64, bytes: Vec<u8>) {
        self.blobs
            .lock()
            .unwrap()
            .insert(name.to_owned(), (version, bytes));
    }
}

impl Blobs for Stand {
    async fn list(&self) -> Result<Vec<Listed>, SyncError> {
        Ok(self
            .blobs
            .lock()
            .unwrap()
            .iter()
            .map(|(name, (version, _))| Listed {
                name: name.clone(),
                version: *version,
            })
            .collect())
    }

    async fn get(&self, name: &str) -> Result<Option<(u64, Vec<u8>)>, SyncError> {
        Ok(self.blobs.lock().unwrap().get(name).cloned())
    }

    async fn put(&self, name: &str, base: u64, bytes: Vec<u8>) -> Result<u64, SyncError> {
        let mut blobs = self.blobs.lock().unwrap();
        if bytes.len() > MOST_BLOB {
            return Err(SyncError::TooLarge(
                "a synced blob is at most 1 MiB".to_owned(),
            ));
        }
        let now = blobs.get(name).map(|(version, _)| *version).unwrap_or(0);
        if now != base {
            return Err(SyncError::Stale);
        }
        let others: usize = blobs
            .iter()
            .filter(|(other, _)| *other != name)
            .map(|(_, (_, bytes))| bytes.len())
            .sum();
        let count = blobs.keys().filter(|other| *other != name).count();
        if others + bytes.len() > MOST_BYTES || count + 1 > MOST_BLOBS {
            return Err(SyncError::TooLarge(
                "this account's room is full".to_owned(),
            ));
        }
        self.sent
            .lock()
            .unwrap()
            .push((name.to_owned(), base + 1, bytes.clone()));
        blobs.insert(name.to_owned(), (base + 1, bytes));
        Ok(base + 1)
    }

    async fn clear(&self) -> Result<(), SyncError> {
        self.blobs.lock().unwrap().clear();
        Ok(())
    }
}

/// A machine: a devpit home with its own store, sync key and state.
pub struct Home {
    pub dir: tempfile::TempDir,
    pub store: Store,
    pub name: String,
}

/// The wall clock, as the app reads it.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or_default()
}

impl Home {
    pub fn new(name: &str) -> Home {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = Store::open(&dir.path().join("state.db")).expect("open");
        Home {
            dir,
            store,
            name: name.to_owned(),
        }
    }

    pub fn root(&self) -> PathBuf {
        self.dir.path().to_owned()
    }

    pub fn key(&self) -> Option<SyncKey> {
        files::key(&self.root())
    }

    pub fn state(&self) -> State {
        files::state(&self.root())
    }

    /// One round, as the app runs it, then the state kept.
    pub async fn sync<B: Blobs>(&mut self, blobs: &B) -> Result<Outcome, SyncError> {
        let now_ms = now_ms();
        let key = self.key().expect("this machine holds the key");
        let mut state = self.state();
        let machine = state.machine.clone();
        let prefs = Prefs { store: &self.store };
        let projects = Projects { store: &self.store };
        let machines = Machines {
            id: &machine,
            name: &self.name,
            now_ms,
        };
        let spaces: [&dyn Namespace; 3] = [&prefs, &projects, &machines];
        let outcome = round(blobs, &key, &mut state, &spaces, now_ms).await;
        files::keep_state(&self.root(), &state).expect("keep state");
        outcome
    }

    pub fn pref(&self, name: &str) -> Option<String> {
        self.store.preference(name).expect("read")
    }
}

/// Every blob's words, opened with `key`, as one text to search.
pub fn opened(key: &SyncKey, kept: &Kept) -> String {
    kept.iter()
        .map(|(name, (version, bytes))| {
            let words =
                devpit_sync::seal::open(key, name, *version, bytes).expect("opens with the key");
            String::from_utf8(words).expect("utf-8")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether `needle` is anywhere in `haystack`, as bytes.
pub fn holds(haystack: &[u8], needle: &str) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle.as_bytes())
}
