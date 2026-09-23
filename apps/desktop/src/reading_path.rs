//! `path.read` — a file named by its full path, for the viewer.
//!
//! A terminal prints paths — an agent's `Read(/…/shot.png)`, a test's log —
//! and a click on one opens it here. Narrower than `path.open`, which only
//! hands a path to the desktop: this sends the bytes to the window, so it
//! reads a project's files, the conversations and pastes devpit keeps, and a
//! CLI installation's skills, commands and transcripts — and never a file
//! whose name says it holds a secret, anywhere.

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::{ErrorCode, FileContents, RpcError};

/// `path.read` — what is at an absolute path, as `file.read` answers.
#[tauri::command]
#[specta::specta]
pub async fn path_read(path: String) -> Result<FileContents, RpcError> {
    crate::off_main::blocking(move || path_read_now(path)).await
}

/// [`path_read`], on the calling thread.
pub(crate) fn path_read_now(path: String) -> Result<FileContents, RpcError> {
    let target = crate::reveal::allowed(&path)?;
    let store = Store::open_default()?;
    let projects: Vec<PathBuf> = store
        .projects()?
        .into_iter()
        .map(|row| PathBuf::from(row.root_path))
        .collect();
    let installations: Vec<PathBuf> = crate::installations::found()
        .unwrap_or_default()
        .into_iter()
        .map(|one| one.directory)
        .collect();
    let home = Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    if !readable(&target, &projects, &installations, &home) {
        return Err(RpcError::new(
            ErrorCode::Forbidden,
            "that file is not one the viewer opens: it may hold a secret",
        ));
    }
    let (Some(folder), Some(name)) = (target.parent(), target.file_name()) else {
        return Err(RpcError::new(ErrorCode::Invalid, "that is not a file"));
    };
    let mut read = crate::files::contents(folder, name.to_string_lossy().into_owned())?;
    // Named as it was asked for, which is what the tab and its title show.
    read.path = path;
    Ok(read)
}

/// A name that says it holds a credential, wherever it is.
pub(crate) fn secret_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == ".credentials.json"
        || (lower.starts_with("settings") && lower.contains(".json"))
        || lower.starts_with(".claude.json")
        || lower.starts_with(".env")
        || lower.starts_with("id_")
        || [".netrc", ".npmrc", ".pypirc", ".git-credentials"].contains(&lower.as_str())
        || lower.ends_with(".pem")
        || lower.ends_with(".key")
}

/// Whether the viewer may read `target`, already resolved and inside one of
/// the roots `path.open` allows.
pub(crate) fn readable(
    target: &Path,
    projects: &[PathBuf],
    installations: &[PathBuf],
    home: &Path,
) -> bool {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if secret_name(&name) {
        return false;
    }
    if projects.iter().any(|root| target.starts_with(root)) {
        return true;
    }
    let first = |root: &Path| {
        target
            .strip_prefix(root)
            .ok()
            .and_then(|rest| rest.components().next())
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
    };
    if let Some(top) = first(home) {
        return ["projects", "worktrees"].contains(&top.as_str());
    }
    installations.iter().any(|root| {
        first(root).is_some_and(|top| {
            [
                "projects",
                "skills",
                "commands",
                "agents",
                "plugins",
                "plans",
                "CLAUDE.md",
            ]
            .contains(&top.as_str())
        })
    })
}

#[cfg(test)]
#[path = "reading_path_tests.rs"]
mod tests;
