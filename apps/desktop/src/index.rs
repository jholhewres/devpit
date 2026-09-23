//! Every file in a project, once, so search does not walk the disk per key.
//!
//! One list handed over and filtered in the window. A monorepo has hundreds
//! of thousands of files and a search that stats them on every keystroke is a
//! search nobody leaves open.

use std::path::Path;

use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::roots::root_of;

/// Never walk past this many. A repository with a million files would
/// otherwise turn opening the palette into a minute of waiting.
const MOST_FILES: usize = 40_000;

/// Directories worth nobody's time. The same list the tree skips.
const SKIP: [&str; 6] = [
    ".git",
    "node_modules",
    "target",
    "dist",
    ".venv",
    "__pycache__",
];

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FileIndex {
    /// Paths relative to the project root.
    pub paths: Vec<String>,
    /// True when the walk stopped at the ceiling. The screen says the list is
    /// partial rather than letting a missing file read as "not there".
    pub partial: bool,
}

fn walk(root: &Path, at: &Path, into: &mut Vec<String>) -> bool {
    if into.len() >= MOST_FILES {
        return false;
    }
    let Ok(entries) = std::fs::read_dir(at) else {
        return true;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') && name != ".github" {
            continue;
        }
        if SKIP.contains(&name.as_str()) {
            continue;
        }
        // `file_type` does not follow the link, which is what we want: a
        // symlinked directory is not walked into, so a loop cannot happen.
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => {
                if !walk(root, &path, into) {
                    return false;
                }
            }
            Ok(kind) if kind.is_file() => {
                if into.len() >= MOST_FILES {
                    return false;
                }
                if let Ok(relative) = path.strip_prefix(root) {
                    into.push(relative.to_string_lossy().into_owned());
                }
            }
            _ => {}
        }
    }
    true
}

/// `project.files` — every file in the project, for the search field.
#[tauri::command]
#[specta::specta]
pub async fn project_files(
    project_id: String,
    worktree_id: Option<String>,
) -> Result<FileIndex, RpcError> {
    crate::off_main::blocking(move || project_files_now(project_id, worktree_id)).await
}

/// [`project_files`], on the calling thread.
pub(crate) fn project_files_now(
    project_id: String,
    worktree_id: Option<String>,
) -> Result<FileIndex, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    let root = root
        .canonicalize()
        .map_err(|err| RpcError::new(ErrorCode::NotFound, err.to_string()))?;

    let mut paths = Vec::new();
    let whole = walk(&root, &root, &mut paths);
    paths.sort();
    Ok(FileIndex {
        partial: !whole,
        paths,
    })
}

#[cfg(test)]
#[path = "index_tests.rs"]
mod tests;
