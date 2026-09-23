//! Browsing the devpit workspace — what the product keeps outside the
//! repository, on disk, where nothing else in the window looks.
//!
//! The Files panel drew the project's checkout, which is what the Explorer
//! beside it already draws. The workspace was the half with no reader: the
//! transcript of every session, the worktree a card actually runs in, the
//! agents this machine has. All of it real, none of it visible.
//!
//! Apart from `workspace.rs` because that one answers "how much room is this
//! taking", a question with four fixed rows and a total. This one answers
//! "what is in here", which is a folder at a time and unbounded.

use std::path::Path;

use devpit_core::home::ProjectHome;
use devpit_core::{tree, Store};
use devpit_rpc::{FileContents, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// One entry in a workspace folder.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceEntry {
    pub name: String,
    /// Relative to the workspace root, `/` separated.
    pub path: String,
    pub is_dir: bool,
    /// Bytes, for a file. Zero for a directory: measuring one means walking
    /// it, and a listing that walks every folder it lists reads the whole
    /// workspace to draw one screen.
    pub bytes: f64,
    /// Milliseconds since the epoch; zero when it could not be read.
    pub modified: f64,
    /// How many entries a directory holds. Absent for a file.
    pub count: Option<u32>,
}

/// A folder worth a shortcut.
///
/// Every folder devpit makes for a project is named by ULID, so the way to
/// this project's own files is thirty characters nobody can recognise, let
/// alone type.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub label: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceListing {
    /// The workspace root, absolute, so the panel can say where this is.
    pub root: String,
    /// The folder listed, relative to the root. Empty at the top.
    pub path: String,
    pub entries: Vec<WorkspaceEntry>,
    /// Only the ones that exist: a shortcut to a folder devpit has not made
    /// yet is a dead end dressed as a feature.
    pub places: Vec<Place>,
}

/// What one entry is, measured now.
fn described(root: &Path, entry: tree::Entry) -> WorkspaceEntry {
    let full = root.join(&entry.path);
    WorkspaceEntry {
        bytes: if entry.is_dir {
            0.0
        } else {
            std::fs::metadata(&full)
                .map(|meta| meta.len())
                .unwrap_or_default() as f64
        },
        modified: crate::files::modified(&full),
        count: entry.is_dir.then(|| {
            std::fs::read_dir(&full)
                .map(|entries| entries.flatten().count() as u32)
                .unwrap_or(0)
        }),
        name: entry.name,
        path: entry.path,
        is_dir: entry.is_dir,
    }
}

/// The folders this project owns, in the order a person would want them.
///
/// Filtered by what is on disk rather than by what the layout says should be:
/// a project that has never run a card has no worktree folder, and offering
/// one teaches the reader the shortcuts lie.
pub(crate) fn places_for(
    root: &Path,
    project: Option<(&str, &ProjectHome)>,
    has: impl Fn(&Path) -> bool,
) -> Vec<Place> {
    let mut places = Vec::new();
    if let Some((id, home)) = project {
        places.push(Place {
            label: "Sessions".to_owned(),
            path: home.relative(),
        });
        places.push(Place {
            label: "Worktrees".to_owned(),
            path: format!("worktrees/{id}"),
        });
    }
    places.push(Place {
        label: "Agents".to_owned(),
        path: "agents".to_owned(),
    });
    places.retain(|place| has(&root.join(&place.path)));
    places
}

/// `workspace.list` — one folder of the devpit workspace.
///
/// `path` absent means "wherever this project's own files are". The panel
/// cannot ask for that itself without a round trip to find out whether the
/// folder exists, and opening on a list of ULIDs is not an answer.
#[tauri::command]
#[specta::specta]
pub async fn workspace_list(
    project_id: Option<String>,
    path: Option<String>,
) -> Result<WorkspaceListing, RpcError> {
    crate::off_main::blocking(move || workspace_list_now(project_id, path)).await
}

/// [`workspace_list`], on the calling thread.
pub(crate) fn workspace_list_now(
    project_id: Option<String>,
    path: Option<String>,
) -> Result<WorkspaceListing, RpcError> {
    let root = Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    let home = project_id
        .as_deref()
        .map(crate::projects::project_home)
        .transpose()?;
    let places = places_for(&root, project_id.as_deref().zip(home.as_ref()), |at| {
        at.exists()
    });
    let path = path.unwrap_or_else(|| {
        places
            .first()
            .map(|place| place.path.clone())
            .unwrap_or_default()
    });

    let entries = tree::children(&root, &path)
        .map_err(crate::filetree::tree_error)?
        .into_iter()
        .map(|entry| described(&root, entry))
        .collect();

    Ok(WorkspaceListing {
        root: root.display().to_string(),
        path,
        entries,
        places,
    })
}

/// `workspace.file` — one file of the workspace, read for preview.
///
/// Through `files::contents`, which resolves the path through symlinks and
/// checks containment before it reads a byte, and refuses anything past its
/// ceiling. A second reader here would be a second place for those to drift.
#[tauri::command]
#[specta::specta]
pub async fn workspace_file(path: String) -> Result<FileContents, RpcError> {
    crate::off_main::blocking(move || workspace_file_now(path)).await
}

/// [`workspace_file`], on the calling thread.
pub(crate) fn workspace_file_now(path: String) -> Result<FileContents, RpcError> {
    let root = Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    // The files devpit keeps owner-only hold its secrets and its store: listed
    // in the workspace, never read into the window.
    let first = path
        .split('/')
        .find(|part| !part.is_empty())
        .unwrap_or_default();
    if devpit_core::home::private_name(first) {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Forbidden,
            "that file holds a secret and is not shown",
        ));
    }
    crate::files::contents(&root, path)
}

#[cfg(test)]
#[path = "wsfiles_tests.rs"]
mod tests;
