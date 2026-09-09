//! Writing a file back, and refusing to lose someone else's change.
//!
//! Apart from reading because the risk is different: a bad read draws the
//! wrong thing, a bad write destroys work that is not on screen.

use devpit_rpc::{ErrorCode, FileSaved, RpcError};

use crate::files::modified;
use crate::roots::root_of;

/// Whether a save is built on a read the file has moved on from.
///
/// A function of its own so the test calls the rule rather than a copy of it:
/// the first version of these tests re-stated the comparison, which meant a
/// change to the rule left them green.
///
/// A file with no recorded mtime on either side is saveable — refusing there
/// would make a new file unsaveable, which is not what this protects.
fn is_stale(read_at: f64, on_disk: f64) -> bool {
    read_at > 0.0 && on_disk > 0.0 && (on_disk - read_at).abs() > 1.0
}

/// `file.write` — saves, and refuses to overwrite a change it never saw.
///
/// `read_at` is the mtime the editor was handed. If the file has moved on
/// since, the save is refused: silently winning that race is how someone
/// loses work they did in another window.
#[tauri::command]
#[specta::specta]
pub fn file_write(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
    text: String,
    read_at: f64,
) -> Result<FileSaved, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    let resolved = devpit_core::tree::resolve(&root, &path)
        .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?;

    if is_stale(read_at, modified(&resolved)) {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            format!("{path} changed on disk since it was opened — reopen it first"),
        ));
    }

    std::fs::write(&resolved, text.as_bytes())
        .map_err(|err| RpcError::internal(err.to_string()))?;

    Ok(FileSaved {
        bytes: text.len() as f64,
        read_at: modified(&resolved),
        path,
    })
}

#[cfg(test)]
#[path = "saves_tests.rs"]
mod tests;
