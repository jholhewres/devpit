//! An orchestrator's settings — the projects linked to it and the account it
//! speaks as — kept here, by its folder.
//!
//! They used to be a file inside that folder, which the orchestrator's own
//! chat edits freely: it could link itself to any project or change account
//! on its own. The store is out of its reach. What an older devpit wrote in the
//! folder is brought in once, the first time it is asked, and not read again.

use std::path::Path;

use serde_json::{json, Value};

use crate::home::ORCHESTRATOR_SETTINGS;
use crate::store::{Store, StoreError};

/// The most of a legacy settings file read: it is a line of JSON.
const LEGACY_MOST: u64 = 4096;

fn key_of(folder: &Path) -> String {
    let folder = folder
        .canonicalize()
        .unwrap_or_else(|_| folder.to_path_buf());
    format!("orchestrator.settings:{}", folder.display())
}

fn object(text: &str) -> Value {
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

/// What the folder's own file said, for the one time it is brought in.
fn legacy(folder: &Path) -> Value {
    let file = folder.join(ORCHESTRATOR_SETTINGS);
    match std::fs::metadata(&file) {
        Ok(meta) if meta.len() <= LEGACY_MOST => {
            object(&std::fs::read_to_string(file).unwrap_or_default())
        }
        _ => json!({}),
    }
}

impl Store {
    /// The settings of the orchestrator in `folder`, as an object.
    pub fn orchestrator_settings(&self, folder: &Path) -> Result<Value, StoreError> {
        let key = key_of(folder);
        if let Some(kept) = self.preference(&key)? {
            return Ok(object(&kept));
        }
        let brought = legacy(folder);
        self.set_preference(&key, &brought.to_string())?;
        Ok(brought)
    }

    /// One setting of the orchestrator in `folder` changed, the rest kept.
    pub fn set_orchestrator_setting(
        &self,
        folder: &Path,
        name: &str,
        value: Value,
    ) -> Result<(), StoreError> {
        let mut all = self.orchestrator_settings(folder)?;
        all[name] = value;
        self.set_preference(&key_of(folder), &all.to_string())
    }
}

#[cfg(test)]
#[path = "orchestrators_tests.rs"]
mod tests;
