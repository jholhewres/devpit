//! `checkpoint.tested` — a run's test report, asked only for an opened run.

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError, Tested, TESTS_EVIDENCE};

/// The run's report, or an empty [`Tested`] when it left none.
pub(crate) fn tested(store: &Store, run_id: &str) -> Result<Tested, RpcError> {
    if store.run_state(run_id)?.is_none() {
        return Err(RpcError::new(ErrorCode::NotFound, "no such run"));
    }
    Ok(store
        .evidence_of(run_id)?
        .filter(|evidence| evidence.version == TESTS_EVIDENCE)
        .and_then(|evidence| serde_json::from_str(&evidence.payload).ok())
        .unwrap_or_default())
}

#[tauri::command]
#[specta::specta]
pub async fn checkpoint_tested(run_id: String) -> Result<Tested, RpcError> {
    crate::off_main::blocking(move || checkpoint_tested_now(run_id)).await
}

/// [`checkpoint_tested`], on the calling thread.
pub(crate) fn checkpoint_tested_now(run_id: String) -> Result<Tested, RpcError> {
    tested(&crate::board::store()?, &run_id)
}

#[cfg(test)]
#[path = "checkpoint_tested_tests.rs"]
mod tests;
