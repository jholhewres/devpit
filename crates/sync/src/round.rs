//! One round of sync: read what devpit.app holds, merge it with what changed
//! here, write the result to this machine, and send devpit.app what it does
//! not have yet.
//!
//! Each namespace keeps a mirror — everything this machine knows of it,
//! from every machine — and the version of devpit.app's copy it last read.
//! A field differing from the mirror changed here and is stamped; a blob
//! whose version moved on is read and merged; whatever wins is written
//! here; and a mirror that holds more than devpit.app's copy is sealed and
//! sent as the next version. Another machine writing first makes the send
//! stale, and the round reads again.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::blobs::{too_large, Blobs, Listed, MOST_BLOB};
use crate::clock::{Clock, Stamp};
use crate::doc::{Cell, Doc};
use crate::error::SyncError;
use crate::key::SyncKey;
use crate::seal::{open, seal, sealed_by, Unsealed};
use crate::space::Namespace;

/// How many times a round reads again after another machine wrote first.
const TRIES: usize = 3;

/// What this machine keeps of one namespace between rounds.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Kept {
    /// devpit.app's version this machine last read or wrote; 0 for none.
    pub version: u64,
    /// The digest of devpit.app's copy at that version.
    pub server: String,
    /// Everything this machine knows of the namespace.
    pub mirror: Doc,
    /// The records this machine had last round: one it did not have is
    /// joining, and takes what the others settled before it adds its own.
    pub seen: BTreeSet<String>,
}

/// This machine's sync, as it is kept on disk between rounds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// Who wrote a stamp: this machine, among the account's.
    pub machine: String,
    pub clock: Clock,
    pub spaces: BTreeMap<String, Kept>,
    /// When a round last went through, in milliseconds.
    #[serde(default)]
    pub synced_at: Option<u64>,
}

impl State {
    pub fn new() -> State {
        let machine = format!("m_{}", ulid::Ulid::generate());
        State {
            clock: Clock::new(&machine),
            machine,
            spaces: BTreeMap::new(),
            synced_at: None,
        }
    }

    pub fn mirror(&self, namespace: &str) -> Option<&Doc> {
        self.spaces.get(namespace).map(|kept| &kept.mirror)
    }

    /// Whether this machine never synced anything yet.
    fn fresh(&self) -> bool {
        self.spaces.values().all(|kept| kept.version == 0)
    }
}

impl Default for State {
    fn default() -> State {
        State::new()
    }
}

/// What a round did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// The namespaces whose values changed on this machine.
    pub written: Vec<String>,
    /// The namespaces sent to devpit.app.
    pub sent: Vec<String>,
    /// What a namespace could not do, while the others went on.
    pub problems: Vec<String>,
}

/// One round over `spaces`, `now_ms` being the wall clock.
pub async fn round<B: Blobs>(
    blobs: &B,
    key: &SyncKey,
    state: &mut State,
    spaces: &[&dyn Namespace],
    now_ms: u64,
) -> Result<Outcome, SyncError> {
    let mut outcome = Outcome::default();
    for tried in 1..=TRIES {
        match once(blobs, key, state, spaces, now_ms, &mut outcome).await {
            Err(SyncError::Stale) if tried < TRIES => continue,
            Err(err) => return Err(err),
            Ok(()) => {
                state.synced_at = Some(now_ms);
                break;
            }
        }
    }
    outcome.written.sort();
    outcome.written.dedup();
    Ok(outcome)
}

async fn once<B: Blobs>(
    blobs: &B,
    key: &SyncKey,
    state: &mut State,
    spaces: &[&dyn Namespace],
    now_ms: u64,
    outcome: &mut Outcome,
) -> Result<(), SyncError> {
    let listed = blobs.list().await?;
    let names: Vec<String> = spaces.iter().map(|ns| key.blob_name(ns.name())).collect();
    if state.fresh() {
        stranger(blobs, key, &listed, &names).await?;
    }
    for (ns, name) in spaces.iter().zip(&names) {
        let kept = state.spaces.entry(ns.name().to_owned()).or_default();
        let fetched = fetch(blobs, key, name, kept, &listed).await?;
        // Kept as soon as it is settled, sent or not: what was written here
        // is in the mirror, and is not taken for a change of this machine's
        // when the round reads again.
        if settle(*ns, kept, &mut state.clock, fetched.as_ref(), now_ms)? {
            outcome.written.push(ns.name().to_owned());
        }
        if kept.mirror.digest() == kept.server {
            continue;
        }
        let sealed =
            seal(key, name, kept.version + 1, &kept.mirror.to_words()).map_err(SyncError::Local)?;
        if sealed.len() > MOST_BLOB {
            // Kept here with what it merged; the others go on.
            outcome.problems.push(too_large(sealed.len()).to_string());
            continue;
        }
        kept.version = blobs.put(name, kept.version, sealed).await?;
        kept.server = kept.mirror.digest();
        outcome.sent.push(ns.name().to_owned());
    }
    Ok(())
}

/// Refuses to sync a fresh machine into an account whose blobs another key
/// sealed: it would start a second copy beside the first, read by nobody.
async fn stranger<B: Blobs>(
    blobs: &B,
    key: &SyncKey,
    listed: &[Listed],
    names: &[String],
) -> Result<(), SyncError> {
    if listed.iter().any(|blob| names.contains(&blob.name)) {
        return Ok(());
    }
    let Some(other) = listed.first() else {
        return Ok(());
    };
    match blobs.get(&other.name).await? {
        Some((_, bytes)) if sealed_by(&bytes).is_some_and(|id| id != key.id_bytes()) => {
            Err(SyncError::OtherKey)
        }
        _ => Ok(()),
    }
}

/// devpit.app's copy, when it moved on since this machine last read it.
async fn fetch<B: Blobs>(
    blobs: &B,
    key: &SyncKey,
    name: &str,
    kept: &Kept,
    listed: &[Listed],
) -> Result<Option<(u64, Doc)>, SyncError> {
    let there = listed
        .iter()
        .find(|blob| blob.name == name)
        .map(|blob| blob.version);
    match there {
        None if kept.version > 0 => Err(SyncError::Reset),
        None => Ok(None),
        Some(version) if version < kept.version => Err(SyncError::Rollback),
        Some(version) if version == kept.version => Ok(None),
        Some(_) => {
            let (version, bytes) = blobs.get(name).await?.ok_or(SyncError::Stale)?;
            if version < kept.version {
                return Err(SyncError::Rollback);
            }
            let words = open(key, name, version, &bytes).map_err(|why| match why {
                Unsealed::OtherKey => SyncError::OtherKey,
                other => SyncError::Unreadable(other.to_string()),
            })?;
            Ok(Some((
                version,
                Doc::from_words(&words).map_err(SyncError::Unreadable)?,
            )))
        }
    }
}

/// Merges what changed here and what devpit.app holds into the mirror, and
/// writes what won to this machine; whether anything was written.
pub(crate) fn settle(
    ns: &dyn Namespace,
    kept: &mut Kept,
    clock: &mut Clock,
    fetched: Option<&(u64, Doc)>,
    now_ms: u64,
) -> Result<bool, SyncError> {
    let here = ns.here().map_err(SyncError::Local)?;
    let mut settled = kept.mirror.clone();
    if let Some((version, doc)) = fetched {
        settled.merge(doc);
        kept.version = *version;
        kept.server = doc.digest();
    }

    // Stamped from the clock as it stood before this look: a change made
    // offline is placed by when it was made, not after what arrived since,
    // nor after another field found changed in the same look.
    let floor = clock.clone();
    let mut mine = Doc::default();
    for (record, fields) in &here {
        let joining = !kept.seen.contains(record);
        for (field, seen) in fields {
            let changed = if joining {
                settled.get(record, field).is_none()
            } else {
                kept.mirror.get(record, field).map(|cell| &cell.value) != Some(&seen.value)
            };
            if changed {
                let at = floor.next(seen.at_ms.unwrap_or(now_ms));
                mine.put(
                    record,
                    field,
                    Cell {
                        value: seen.value.clone(),
                        at,
                    },
                );
            }
        }
    }
    settled.merge(&mine);
    let stamps: Vec<Stamp> = settled.stamps().cloned().collect();
    for stamp in &stamps {
        clock.observe(stamp);
    }

    let mut written = false;
    for (record, fields) in &settled.records {
        let Some(holds) = here.get(record) else {
            continue;
        };
        for (field, cell) in fields {
            let now = holds.get(field).map(|seen| &seen.value);
            if now == Some(&cell.value) || (now.is_none() && cell.value.is_none()) {
                continue;
            }
            ns.write(record, field, cell.value.as_deref())
                .map_err(SyncError::Local)?;
            written = true;
        }
    }
    kept.seen = here.keys().cloned().collect();
    kept.mirror = settled;
    Ok(written)
}

#[cfg(test)]
#[path = "round_tests.rs"]
mod tests;
