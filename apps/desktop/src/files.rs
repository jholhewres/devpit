//! Reading and writing a file inside a project.
//!
//! Every path goes through `tree::resolve`, which follows symlinks first and
//! then checks containment against the project root. This process runs
//! terminals and writes files: reaching it is reaching the machine, and a
//! relative path from a screen is not a path to trust.

use std::path::{Path, PathBuf};

use quockpit_core::Store;
use quockpit_rpc::{ErrorCode, FileContents, FileSaved, RpcError};

/// Read no more than this in one go.
///
/// A file past it is refused with its size rather than truncated: half a file
/// in an editor is a file about to be saved with the other half gone.
const MOST_BYTES: u64 = 2 * 1024 * 1024;

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

fn root_of(project_id: &str, worktree_id: Option<&str>) -> Result<PathBuf, RpcError> {
    let store = store()?;
    let row = store
        .project(project_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that project is not registered"))?;
    let root = PathBuf::from(&row.root_path);
    if let Some(id) = worktree_id {
        if let Some(path) = quockpit_git::worktree_path(&root, id)
            .map_err(|err| RpcError::internal(err.to_string()))?
        {
            return Ok(path);
        }
    }
    Ok(root)
}

fn modified(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_millis() as f64)
        .unwrap_or_default()
}

/// `file.read` — the text of a file, or why it is not text.
#[tauri::command]
#[specta::specta]
pub fn file_read(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<FileContents, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    let resolved = quockpit_core::tree::resolve(&root, &path)
        .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?;

    let bytes = std::fs::metadata(&resolved)
        .map(|meta| meta.len())
        .unwrap_or_default();

    let not_shown = if bytes > MOST_BYTES {
        Some(format!(
            "{} is {:.1} MB — past the {} MB this opens",
            path,
            bytes as f64 / 1_048_576.0,
            MOST_BYTES / 1_048_576
        ))
    } else {
        None
    };

    let text = match not_shown {
        Some(_) => None,
        None => match std::fs::read(&resolved) {
            // Refused rather than rendered: a megabyte of bytes drawn as
            // replacement characters is worse than a sentence saying it is not
            // text, and saving it back would corrupt the file.
            Ok(raw) => String::from_utf8(raw).ok(),
            Err(err) => return Err(RpcError::internal(err.to_string())),
        },
    };

    let not_shown = match (&text, not_shown) {
        (None, None) => Some(format!("{path} is not text")),
        (_, given) => given,
    };

    Ok(FileContents {
        path,
        text,
        not_shown,
        bytes: bytes as f64,
        read_at: modified(&resolved),
    })
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
    let resolved = quockpit_core::tree::resolve(&root, &path)
        .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?;

    let on_disk = modified(&resolved);
    if read_at > 0.0 && on_disk > 0.0 && (on_disk - read_at).abs() > 1.0 {
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
#[path = "files_tests.rs"]
mod tests;
