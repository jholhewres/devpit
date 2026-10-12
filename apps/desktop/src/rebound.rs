//! The keys a person recorded in place of a shortcut's defaults.
//!
//! The registry of shortcuts is the window's (`web/src/shell/shortcuts.ts`);
//! this only keeps what was chosen, with the other preferences, so it is in
//! the store's backups and not in one WebKit profile's storage. An id the
//! window no longer has is simply never consulted.

use std::collections::BTreeMap;

use devpit_core::store::preference;
use devpit_rpc::RpcError;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Rebound {
    /// Shortcut id to the keys it answers to instead of its defaults, as the
    /// window spells them (`Mod+Shift+K`). Absent is the defaults.
    pub keys: BTreeMap<String, Vec<String>>,
}

impl Rebound {
    /// Without the entries that say nothing: an empty list or an empty key
    /// would otherwise be a shortcut with no key at all.
    fn held(mut self) -> Self {
        for keys in self.keys.values_mut() {
            keys.retain(|key| !key.trim().is_empty());
        }
        self.keys.retain(|_, keys| !keys.is_empty());
        self
    }
}

fn read(store: &devpit_core::Store) -> Rebound {
    store
        .preference(preference::SHORTCUT_KEYS)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str::<Rebound>(&raw).ok())
        .unwrap_or_default()
        .held()
}

/// `keys.rebound` — the keys recorded in place of the defaults.
#[tauri::command]
#[specta::specta]
pub async fn rebound_read() -> Result<Rebound, RpcError> {
    crate::off_main::blocking(|| Ok(read(&crate::projects::store()?))).await
}

/// `keys.rebound_set` — replaces them all. The window holds the whole map and
/// changes it at once, so a write is the whole map and not one entry.
#[tauri::command]
#[specta::specta]
pub async fn rebound_set(rebound: Rebound) -> Result<Rebound, RpcError> {
    crate::off_main::blocking(move || rebound_set_now(&crate::projects::store()?, rebound)).await
}

/// [`rebound_set`], on the calling thread.
pub(crate) fn rebound_set_now(
    store: &devpit_core::Store,
    rebound: Rebound,
) -> Result<Rebound, RpcError> {
    let written = serde_json::to_string(&rebound.held())
        .map_err(|err| RpcError::internal(err.to_string()))?;
    store.set_preference(preference::SHORTCUT_KEYS, &written)?;
    crate::sync::nudge();
    Ok(read(store))
}

#[cfg(test)]
#[path = "rebound_tests.rs"]
mod tests;
