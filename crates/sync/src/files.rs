//! What sync keeps in devpit's home on this machine: the key, readable by
//! the person alone, and the state between rounds beside it. Neither goes
//! anywhere else; the key never does.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::key::SyncKey;
use crate::round::State;

fn key_file(root: &Path) -> PathBuf {
    root.join("sync-key.json")
}

fn state_file(root: &Path) -> PathBuf {
    root.join("sync-state.json")
}

#[derive(Serialize, Deserialize)]
struct KeptKey {
    key: String,
}

/// The key this machine holds, if it holds one.
pub fn key(root: &Path) -> Option<SyncKey> {
    let text = std::fs::read_to_string(key_file(root)).ok()?;
    let kept: KeptKey = serde_json::from_str(&text).ok()?;
    SyncKey::from_hex(&kept.key).ok()
}

/// Keeps `key` at mode 600, in place of any before it.
pub fn keep_key(root: &Path, key: &SyncKey) -> Result<(), String> {
    let text =
        serde_json::to_string(&KeptKey { key: key.to_hex() }).map_err(|err| err.to_string())?;
    std::fs::create_dir_all(root).map_err(|err| err.to_string())?;
    devpit_core::home::write_private(&key_file(root), text.as_bytes())
        .map_err(|err| format!("the sync key could not be kept: {err}"))
}

/// This machine's state between rounds; a new one when there is none.
pub fn state(root: &Path) -> State {
    std::fs::read_to_string(state_file(root))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn keep_state(root: &Path, state: &State) -> Result<(), String> {
    let text = serde_json::to_string(state).map_err(|err| err.to_string())?;
    devpit_core::home::write_private(&state_file(root), text.as_bytes())
        .map_err(|err| format!("sync's state could not be kept: {err}"))
}

/// Forgets the key and every mirror: this machine starts again, as one that
/// never synced. Its id and its clock are kept, so the others list it once
/// and its next stamp is still after its last.
pub fn forget(root: &Path) {
    let was = state(root);
    let _ = std::fs::remove_file(key_file(root));
    let _ = keep_state(
        root,
        &State {
            machine: was.machine,
            clock: was.clock,
            spaces: Default::default(),
            synced_at: None,
        },
    );
}
