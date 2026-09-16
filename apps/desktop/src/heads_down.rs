//! The focus that is on, and how long it has been.
//!
//! `focus` already means focusing a tab or a pane in this window, so the
//! identifier here is `heads_down` and the word on screen is "Focus".
//!
//! Only two things live in the store: which project, and the second it began.
//! Everything else a focus does — what it holds back, what it says on the way
//! out — is read from what is already recorded elsewhere. A focus is interface
//! state and one rule, not an eighth object.

use devpit_core::store::preference;
use devpit_rpc::{HeadsDown, RpcError};

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs_f64())
        .unwrap_or_default()
}

/// What a stored focus says, or nothing when the row is absent or spoiled.
///
/// A row that cannot be read is treated as no focus rather than as an error:
/// the app has to open, and a focus nobody can parse is one nobody is in.
pub(crate) fn parse(held: Option<&str>) -> Option<HeadsDown> {
    let (project_id, since) = held?.split_once(':')?;
    if project_id.is_empty() {
        return None;
    }
    Some(HeadsDown {
        project_id: project_id.to_owned(),
        since: since.parse().ok()?,
    })
}

/// `focus.read` — the focus that is on, if one is.
#[tauri::command]
#[specta::specta]
pub fn focus_read() -> Result<Option<HeadsDown>, RpcError> {
    let store = crate::projects::store()?;
    Ok(parse(store.preference(preference::HEADS_DOWN)?.as_deref()))
}

/// `focus.write` — begins a focus on a project, or ends the one that is on.
///
/// Beginning one while another is on replaces it: changing project ends the
/// focus, and the window asks for the new one in the same breath.
#[tauri::command]
#[specta::specta]
pub fn focus_write(project_id: Option<String>) -> Result<Option<HeadsDown>, RpcError> {
    let store = crate::projects::store()?;
    let Some(project_id) = project_id.filter(|id| !id.is_empty()) else {
        store.set_preference(preference::HEADS_DOWN, "")?;
        return Ok(None);
    };

    // The clock is the store's, not the window's: the elapsed time has to
    // survive a restart, and a counter in memory does not.
    let began = HeadsDown {
        project_id,
        since: now(),
    };
    // Whole seconds on the way out: the row is read back by a person as often
    // as by the app, and a fractional second in it says nothing.
    store.set_preference(
        preference::HEADS_DOWN,
        &format!("{}:{}", began.project_id, began.since as i64),
    )?;
    Ok(Some(began))
}

#[cfg(test)]
#[path = "heads_down_tests.rs"]
mod tests;
