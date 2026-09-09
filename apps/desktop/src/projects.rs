//! Contract commands for projects, adapted to Tauri's transport.
//!
//! Thin on purpose: this layer translates, it does not decide. The row comes
//! from `crates/core`, the repository from `crates/git`, the shape from
//! `crates/rpc`, and this file only puts them in the same envelope.

use std::path::{Path, PathBuf};

use devpit_core::{tree, Store};
use devpit_rpc::{
    ErrorCode, FileNode, GitStatus, Note, Project, ProjectChanges, ProjectHistory, ProjectList,
    ProjectNotes, ProjectTree, RpcError,
};

/// How many commits the overview asks for.
///
/// A number, not "all": `git log` on a large repository walks the whole graph,
/// and the surface shows four.
const HISTORY: u32 = 8;

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

/// Reads the project row and the folder it points at.
///
/// Both, because every command below needs both and a project whose folder has
/// been moved or deleted must fail as `not_found` with a sentence, rather than
/// as a filesystem error nobody can act on.
fn locate(store: &Store, id: &str) -> Result<(devpit_core::ProjectRow, PathBuf), RpcError> {
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

fn tree_error(err: devpit_core::TreeError) -> RpcError {
    match err {
        // Its own code, not a generic one: the screen says something different
        // for a path that escaped than for a folder it could not read, and
        // this process runs terminals — reaching it is reaching the machine.
        devpit_core::TreeError::Outside { .. } => RpcError::forbidden(err.to_string()),
        other => RpcError::internal(other.to_string()),
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
        .map(|row| {
            let root = PathBuf::from(&row.root_path);
            let (worktrees, unreadable) = match devpit_git::worktrees(&root) {
                Ok(found) => (found, None),
                Err(err) => (Vec::new(), Some(err.to_string())),
            };

            Project {
                id: row.id,
                name: row.name,
                root_path: row.root_path,
                group: row.group,
                accent: row.accent,
                worktrees,
                unreadable,
            }
        })
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

    let (worktrees, unreadable) = match devpit_git::worktrees(&root) {
        Ok(found) => (found, None),
        Err(err) => (Vec::new(), Some(err.to_string())),
    };

    let row = store
        .project(&id)?
        .ok_or_else(|| RpcError::internal("the project vanished between write and read"))?;

    Ok(Project {
        id: row.id,
        name: row.name,
        root_path: row.root_path,
        group: row.group,
        accent: row.accent,
        worktrees,
        unreadable,
    })
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

    let (worktrees, unreadable) = match devpit_git::worktrees(&into) {
        Ok(found) => (found, None),
        Err(err) => (Vec::new(), Some(err.to_string())),
    };

    Ok(Project {
        id: row.id,
        name: row.name,
        root_path: row.root_path,
        group: row.group,
        accent: row.accent,
        worktrees,
        unreadable,
    })
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

/// `project.tree` — one level of the file tree, from a given worktree.
///
/// One level rather than the whole tree: a monorepo has hundreds of thousands
/// of files and the screen draws only what is expanded. `path` is empty for
/// the root.
#[tauri::command]
#[specta::specta]
pub fn project_tree(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<ProjectTree, RpcError> {
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let root = checkout(&root, worktree_id.as_deref());

    let status = devpit_git::status(&root)
        .map(|status| status.paths)
        .unwrap_or_default();

    let nodes = tree::children(&root, &path)
        .map_err(tree_error)?
        .into_iter()
        .map(|entry| FileNode {
            status: mark(&entry, &status),
            children: entry.is_dir.then(Vec::new),
            name: entry.name,
            path: entry.path,
        })
        .collect();

    Ok(ProjectTree { nodes })
}

/// How much a status wants to be seen, when several are collapsed into one row.
///
/// A deletion outranks a modification outranks an addition: the further left,
/// the harder it is to undo by accident. Untracked is last because it is the
/// normal state of a working tree, not news.
fn loudness(status: GitStatus) -> u8 {
    match status {
        GitStatus::Deleted => 4,
        GitStatus::Modified => 3,
        GitStatus::Added => 2,
        GitStatus::Untracked => 1,
        GitStatus::Clean => 0,
    }
}

/// A directory shows the loudest thing under it.
///
/// Otherwise a change three levels down is invisible until you have opened
/// three folders looking for it, which is the opposite of what the colour is
/// for. Ranked rather than first-found: the map is ordered by path, and taking
/// its first entry would show whichever file happens to sort earliest.
fn mark(entry: &tree::Entry, status: &std::collections::BTreeMap<String, GitStatus>) -> GitStatus {
    if !entry.is_dir {
        return status.get(&entry.path).copied().unwrap_or(GitStatus::Clean);
    }

    let prefix = format!("{}/", entry.path);
    status
        .iter()
        .filter(|(path, _)| path.starts_with(&prefix))
        .map(|(_, status)| *status)
        .max_by_key(|status| loudness(*status))
        .unwrap_or(GitStatus::Clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(path: &str) -> tree::Entry {
        tree::Entry {
            name: path.rsplit('/').next().unwrap_or(path).to_owned(),
            path: path.to_owned(),
            is_dir: true,
        }
    }

    #[test]
    fn a_directory_shows_its_loudest_change_not_its_first() {
        // `a.rs` sorts before `z.rs`, so first-found would report Modified and
        // the deletion under the same folder would go unseen.
        let status = std::collections::BTreeMap::from([
            ("web/a.rs".to_owned(), GitStatus::Modified),
            ("web/z.rs".to_owned(), GitStatus::Deleted),
        ]);
        assert_eq!(mark(&dir("web"), &status), GitStatus::Deleted);
    }

    #[test]
    fn a_directory_with_nothing_under_it_is_clean() {
        let status = std::collections::BTreeMap::from([
            // Same prefix, different folder: `webbing` must not count as `web`.
            ("webbing/a.rs".to_owned(), GitStatus::Modified),
        ]);
        assert_eq!(mark(&dir("web"), &status), GitStatus::Clean);
    }
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

/// `project.history` — the last few commits of a checkout.
#[tauri::command]
#[specta::specta]
pub fn project_history(
    project_id: String,
    worktree_id: Option<String>,
) -> Result<ProjectHistory, RpcError> {
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let root = checkout(&root, worktree_id.as_deref());

    let commits =
        devpit_git::history(&root, HISTORY).map_err(|err| RpcError::internal(err.to_string()))?;

    Ok(ProjectHistory { commits })
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
fn checkout(root: &Path, worktree_id: Option<&str>) -> PathBuf {
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
