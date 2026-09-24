//! devpit's own errors, for the report the person may switch on.
//!
//! What is kept and how is `devpit_core::reports`; this is where the app
//! starts it and where the window hands over what it caught itself.

use std::path::Path;

use devpit_core::{preference, reports, Store, StoreError};
use devpit_rpc::RpcError;

/// Picks the switch up where it was left, once the home exists.
pub(crate) fn resume(root: &Path) {
    resume_with(Store::open_default(), root);
}

/// [`resume`], given the store. Off wipes too: a file a failed wipe left
/// behind goes at the next start, and on prunes what aged out while closed.
pub(crate) fn resume_with(store: Result<Store, StoreError>, root: &Path) {
    let on = store
        .and_then(|store| store.preference_flag(preference::ERROR_REPORTS))
        .ok()
        .flatten()
        .unwrap_or(false);
    let _ = reports::switch(root, on);
}

/// The person's choice, applied before it is recorded: a wipe that failed
/// leaves the switch as it was, not a preference saying off over a file
/// still there.
pub(crate) fn choose(store: &Store, root: &Path, kept: bool) -> Result<(), RpcError> {
    reports::switch(root, kept)
        .map_err(|err| RpcError::internal(format!("the error file could not be removed: {err}")))?;
    store.set_preference_flag(preference::ERROR_REPORTS, kept)?;
    Ok(())
}

/// `errors.report` — an error the window caught and nothing else handled.
///
/// Accepted whether or not reports are on: off, it is dropped here rather
/// than the window having to know the switch.
#[tauri::command]
#[specta::specta]
pub async fn errors_report(message: String, stack: Option<String>) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        from_window(&message, stack.as_deref());
        Ok(())
    })
    .await
}

/// [`errors_report`], on the calling thread.
pub(crate) fn from_window(message: &str, stack: Option<&str>) {
    reports::record(reports::Report {
        kind: "frontend",
        location: None,
        message,
        stack,
    });
}

#[cfg(test)]
#[path = "error_reports_tests.rs"]
mod tests;
