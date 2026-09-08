//! What is uncommitted in one file, right now.
//!
//! A different question from a card's front, which asks what changed on that
//! line of work. This one is the Changes panel: the diff against HEAD, in the
//! checkout as it stands.

use quockpit_rpc::{ErrorCode, RpcError};

use crate::roots::root_of;

/// `file.diff` — what is uncommitted in one file, right now.
///
/// A different question from a card's front, which asks what changed on that
/// line of work. This one is the Changes panel: the diff against HEAD, in the
/// checkout as it stands.
#[tauri::command]
#[specta::specta]
pub fn file_diff(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<String, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    // Resolved and contained like every other path here, even though git would
    // refuse most escapes itself: the check belongs where the path arrives.
    quockpit_core::tree::resolve(&root, &path)
        .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?;

    quockpit_git::diff_file(&root, &path).map_err(|err| RpcError::internal(err.to_string()))
}
