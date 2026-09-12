//! Contract commands for projects, adapted to Tauri's transport.
//!
//! Thin on purpose: this layer translates, it does not decide. The row comes
//! from `crates/core`, the repository from `crates/git`, the shape from
//! `crates/rpc`, and this file only puts them in the same envelope.

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::{ErrorCode, Note, Project, ProjectChanges, ProjectList, ProjectNotes, RpcError};

pub(crate) fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

/// Reads the project row and the folder it points at.
///
/// Both, because every command below needs both and a project whose folder has
/// been moved or deleted must fail as `not_found` with a sentence, rather than
/// as a filesystem error nobody can act on.
pub(crate) fn locate(
    store: &Store,
    id: &str,
) -> Result<(devpit_core::ProjectRow, PathBuf), RpcError> {
    let row = store
        .project(id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that project is not registered"))?;

    let root = PathBuf::from(&row.root_path);
    if !root.is_dir() {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("{} is no longer on disk", row.root_path),
        ));
    }

    Ok((row, root))
}

/// A row, with what git knows about the folder it points at.
///
/// One function rather than the same block in four commands: `Project` gained
/// two fields and three of the four copies would have compiled without them
/// had they been optional, which is how a list ends up disagreeing with the
/// row that was just added to it.
fn drawn(store: &Store, row: devpit_core::ProjectRow) -> Project {
    let root = PathBuf::from(&row.root_path);
    let hidden = crate::sources::hidden(store);
    let (worktrees, unreadable) =
        match devpit_git::worktrees(&root, &crate::sources::ours(store, &root)) {
            // Filtered here rather than on the screen: the count on the row
            // and the list behind it are the same question, and two places
            // applying the same rule is two places to get it wrong.
            Ok(found) => (
                found
                    .into_iter()
                    .filter(|worktree| devpit_git::shown(&hidden, worktree.origin))
                    .collect(),
                None,
            ),
            Err(err) => (Vec::new(), Some(err.to_string())),
        };

    Project {
        id: row.id,
        name: row.name,
        root_path: row.root_path,
        group: row.group,
        accent: row.accent,
        origin: row.origin,
        last_opened_at: row.last_opened_at.map(|at| at as f64),
        worktrees,
        unreadable,
    }
}

/// `project.list` — every registered project with its checkouts.
///
/// A repository that cannot be read does not drop out of the list; it comes
/// back with `unreadable` set and no worktrees. A project missing from the
/// list would read as one that was never added.
#[tauri::command]
#[specta::specta]
pub fn project_list() -> Result<ProjectList, RpcError> {
    let store = store()?;

    let projects = store
        .projects()?
        .into_iter()
        .map(|row| drawn(&store, row))
        .collect();

    Ok(ProjectList { projects })
}

/// `project.add` — registers a folder, or opens the one already registered.
#[tauri::command]
#[specta::specta]
pub fn project_add(root_path: String) -> Result<Project, RpcError> {
    let root = PathBuf::from(shellexpand_home(&root_path));
    if !root.is_dir() {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("{} is not a folder", root.display()),
        ));
    }

    // Canonical, so the same folder reached by two different paths is one
    // project. `root_path` is unique in the schema and would otherwise refuse
    // the second spelling as a duplicate of nothing.
    let root = root
        .canonicalize()
        .map_err(|err| RpcError::internal(err.to_string()))?;

    let store = store()?;
    let origin = origin_url(&root);
    let id = store.add_project(&root, origin.as_deref())?;

    let row = store
        .project(&id)?
        .ok_or_else(|| RpcError::internal("the project vanished between write and read"))?;

    Ok(drawn(&store, row))
}

/// `project.clone` — clones a remote and registers where it landed.
///
/// `into` is the parent folder, and it is optional: someone deciding *whether*
/// to add a project should not be stopped to answer *where*. Left out, it goes
/// to `~/.devpit/repos/`, and the path is shown before the clone runs.
#[tauri::command]
#[specta::specta]
pub fn project_clone(url: String, into: Option<String>) -> Result<Project, RpcError> {
    // Somewhere of their choosing when they chose one. The app's own folder is
    // the answer to "I do not want to decide", not a place to be put.
    let parent = match into.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
        Some(chosen) => std::path::PathBuf::from(chosen),
        None => Store::root()?.join("repos"),
    };

    let into = devpit_git::clone(url.trim(), &parent).map_err(|err| match err {
        devpit_git::GitError::Missing => RpcError::new(ErrorCode::Unsupported, err.to_string()),
        // A clone that failed because the folder is taken is a conflict the
        // person can act on, not an internal error.
        devpit_git::GitError::Failed { ref stderr, .. } if stderr.contains("already exists") => {
            RpcError::new(ErrorCode::Conflict, err.to_string())
        }
        other => RpcError::internal(other.to_string()),
    })?;

    let store = store()?;
    let id = store.add_project(&into, Some(url.trim()))?;
    let row = store
        .project(&id)?
        .ok_or_else(|| RpcError::internal("the project vanished between write and read"))?;

    Ok(drawn(&store, row))
}

/// `project.open` — records that this is the project being worked in.
///
/// It is what orders the list, so it is a write and not a read: the order you
/// see next time is the order you built by using it.
#[tauri::command]
#[specta::specta]
pub fn project_open(project_id: String) -> Result<ProjectList, RpcError> {
    let store = store()?;
    locate(&store, &project_id)?;
    store.touch_project(&project_id)?;
    project_list()
}

/// `project.forget` — takes a project out of the list.
///
/// The folder, its git and its worktrees are untouched: this only stops
/// devpit listing it. Answers with the list that is left, so the screen does
/// not have to guess which project it is standing in now.
///
/// `wipe_workspace` is the box in the dialog, and it was decorative: the
/// checkbox said the board and the per-project settings would go, and nothing
/// went. Ticked, the row and everything hanging off it are deleted and the
/// project's folder under `~/.devpit/projects/` is removed. The repository is
/// still never touched — that is the one promise this command makes.
#[tauri::command]
#[specta::specta]
pub fn project_forget(project_id: String, wipe_workspace: bool) -> Result<ProjectList, RpcError> {
    let store = store()?;

    if !wipe_workspace {
        if !store.forget_project(&project_id)? {
            return Err(RpcError::new(ErrorCode::NotFound, "no such project"));
        }
        return project_list();
    }

    // Before the row, because the row is what says which folder is this
    // project's: erasing first would leave the directory with nothing left to
    // name it.
    let workspace = workspace_of(&project_id)?;
    if !store.erase_project(&project_id)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such project"));
    }
    if workspace.is_dir() {
        std::fs::remove_dir_all(&workspace)
            .map_err(|err| RpcError::internal(format!("{}: {err}", workspace.display())))?;
    }

    project_list()
}

/// Where a project's own workspace lives, checked before anything deletes it.
///
/// The id reaches a `remove_dir_all`, so it is built into the path rather than
/// interpolated from whatever arrived: one path segment, from the alphabet ids
/// are made of, and the result has to still be under the workspace root.
fn workspace_of(project_id: &str) -> Result<PathBuf, RpcError> {
    let sane = !project_id.is_empty()
        && project_id.len() <= 64
        && project_id
            .chars()
            .all(|letter| letter.is_ascii_alphanumeric() || letter == '_' || letter == '-');
    if !sane {
        return Err(RpcError::new(ErrorCode::Invalid, "not a project id"));
    }

    let home = Store::root()?;
    let mine = home.join("projects").join(project_id);
    if !mine.starts_with(home.join("projects")) {
        return Err(RpcError::new(ErrorCode::Forbidden, "not a project id"));
    }
    Ok(mine)
}

/// `project.rename` — what this project is called in devpit.
///
/// The name is the app's, not git's: the folder on disk keeps whatever it was
/// called, because renaming somebody's checkout is not a thing a list should
/// do to make its own rows read better.
#[tauri::command]
#[specta::specta]
pub fn project_rename(project_id: String, name: String) -> Result<ProjectList, RpcError> {
    let wanted = name.trim();
    if wanted.is_empty() {
        return Err(RpcError::new(ErrorCode::Invalid, "a project needs a name"));
    }
    if wanted.chars().count() > 120 {
        return Err(RpcError::new(ErrorCode::Invalid, "that name is too long"));
    }

    let store = store()?;
    if !store.rename_project(&project_id, wanted)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such project"));
    }
    project_list()
}

/// `project.changes` — what has changed in a checkout, with the size of each edit.
#[tauri::command]
#[specta::specta]
pub fn project_changes(
    project_id: String,
    worktree_id: Option<String>,
) -> Result<ProjectChanges, RpcError> {
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let root = checkout(&root, worktree_id.as_deref());

    let changes = devpit_git::changes(&root).map_err(|err| RpcError::internal(err.to_string()))?;

    // Summed here rather than on the screen, so the totals stay right if this
    // list is ever paged.
    let added = changes.iter().map(|change| change.added).sum();
    let removed = changes.iter().map(|change| change.removed).sum();

    Ok(ProjectChanges {
        changes,
        added,
        removed,
    })
}

/// `project.notes` — the notes pinned to a project.
#[tauri::command]
#[specta::specta]
pub fn project_notes(project_id: String) -> Result<ProjectNotes, RpcError> {
    let store = store()?;
    locate(&store, &project_id)?;

    let notes = store
        .notes(&project_id)?
        .into_iter()
        .map(|row| Note {
            id: row.id,
            body: row.body,
            created_at: row.created_at as f64,
        })
        .collect();

    Ok(ProjectNotes { notes })
}

/// `project.note_add` — capture, in one keystroke and no form.
#[tauri::command]
#[specta::specta]
pub fn project_note_add(project_id: String, body: String) -> Result<ProjectNotes, RpcError> {
    let store = store()?;
    locate(&store, &project_id)?;

    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "an empty note is not a note",
        ));
    }

    store.add_note(&project_id, trimmed)?;
    project_notes(project_id)
}

/// Resolves which checkout a command is about.
///
/// Falls back to the project root when the caller names no worktree, which is
/// what a screen that has not loaded the list yet does, and when the named one
/// has since been removed — a stale id should reopen the project, not fail.
pub(crate) fn checkout(root: &Path, worktree_id: Option<&str>) -> PathBuf {
    worktree_id
        .and_then(|id| devpit_git::worktree_path(root, id).ok().flatten())
        .unwrap_or_else(|| root.to_path_buf())
}

/// Reads `remote.origin.url`, which is what makes two clones one project.
fn origin_url(root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "--get", "remote.origin.url"])
        .output()
        .ok()?;

    let url = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (!url.is_empty()).then_some(url)
}

/// Expands a leading `~`, which is how people type a path.
fn shellexpand_home(raw: &str) -> String {
    let trimmed = raw.trim();
    let Some(rest) = trimmed.strip_prefix('~') else {
        return trimmed.to_owned();
    };
    match dirs_home() {
        Some(home) => format!("{}{}", home.display(), rest),
        None => trimmed.to_owned(),
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The id reaches `remove_dir_all`, so it has to be one path segment.
    ///
    /// Not a theory: `workspace_of` builds the path this command deletes, and
    /// a `..` allowed through it is a delete of `~/.devpit` itself.
    #[test]
    fn a_project_id_that_could_climb_out_is_refused() {
        for climbing in ["..", "../..", "a/b", "/etc", "a\0b", ""] {
            assert!(
                workspace_of(climbing).is_err(),
                "{climbing:?} was accepted as a project id"
            );
        }
    }

    #[test]
    fn a_real_project_id_lands_under_the_workspace() {
        let Ok(mine) = workspace_of("prj_01JABCDEF") else {
            // No home directory in this environment; nothing to assert about.
            return;
        };
        assert!(mine.ends_with("projects/prj_01JABCDEF"));
    }
}
