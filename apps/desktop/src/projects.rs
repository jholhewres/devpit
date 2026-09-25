//! Contract commands for projects, adapted to Tauri's transport.
//!
//! Thin on purpose: this layer translates, it does not decide. The row comes
//! from `crates/core`, the repository from `crates/git`, the shape from
//! `crates/rpc`, and this file only puts them in the same envelope.

use std::path::{Path, PathBuf};

use devpit_core::home::{HomeError, ProjectHome, FOLDER_NOTICE};
use devpit_core::Store;
use devpit_rpc::{ErrorCode, Project, ProjectChanges, ProjectList, RpcError};

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
pub(crate) fn drawn(store: &Store, row: devpit_core::ProjectRow) -> Project {
    let root = PathBuf::from(&row.root_path);
    let hidden = crate::sources::hidden(store);
    let orchestrator = devpit_core::Store::root()
        .ok()
        .and_then(|home| home.canonicalize().ok())
        .and_then(|home| devpit_core::home::orchestrator_of(&home, &root));
    // An orchestrator's folder is notes, not a repository: no checkouts to
    // list, and no git to be unreadable for want of.
    let (worktrees, unreadable) = if orchestrator.is_some() {
        (Vec::new(), None)
    } else {
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
        }
    };

    Project {
        id: row.id,
        name: row.name,
        root_path: row.root_path,
        group: row.group,
        accent: row.accent,
        origin: row.origin,
        last_opened_at: row.last_opened_at.map(|at| at as f64),
        icon: row.icon,
        color: row.color,
        orchestrator,
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
pub async fn project_list() -> Result<ProjectList, RpcError> {
    crate::off_main::blocking(project_list_now).await
}

/// [`project_list`], on the calling thread.
pub(crate) fn project_list_now() -> Result<ProjectList, RpcError> {
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
pub async fn project_add(root_path: String) -> Result<Project, RpcError> {
    crate::off_main::blocking(move || project_add_now(root_path)).await
}

/// [`project_add`], on the calling thread.
pub(crate) fn project_add_now(root_path: String) -> Result<Project, RpcError> {
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

/// `project.open` — records that this is the project being worked in.
///
/// It is what orders the list, so it is a write and not a read: the order you
/// see next time is the order you built by using it.
#[tauri::command]
#[specta::specta]
pub async fn project_open(project_id: String) -> Result<ProjectList, RpcError> {
    crate::off_main::blocking(move || project_open_now(project_id)).await
}

/// [`project_open`], on the calling thread.
pub(crate) fn project_open_now(project_id: String) -> Result<ProjectList, RpcError> {
    let store = store()?;
    locate(&store, &project_id)?;
    store.touch_project(&project_id)?;
    project_list_now()
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
pub async fn project_forget(
    project_id: String,
    wipe_workspace: bool,
) -> Result<ProjectList, RpcError> {
    crate::off_main::blocking(move || project_forget_now(project_id, wipe_workspace)).await
}

/// [`project_forget`], on the calling thread.
pub(crate) fn project_forget_now(
    project_id: String,
    wipe_workspace: bool,
) -> Result<ProjectList, RpcError> {
    let store = store()?;

    if !wipe_workspace {
        if !store.forget_project(&project_id)? {
            return Err(RpcError::new(ErrorCode::NotFound, "no such project"));
        }
        return project_list_now();
    }

    erase(&store, &Store::root()?, &project_id)?;
    project_list_now()
}

/// Erases the row and deletes the project's folder under the workspace.
///
/// A stored folder `ProjectHome` refuses deletes nothing, but it does not keep
/// the row either: the bell says the folder was left.
pub(crate) fn erase(store: &Store, root: &Path, project_id: &str) -> Result<(), RpcError> {
    // Before the row, because the row is what says which folder is this
    // project's: erasing first would leave the directory with nothing left to
    // name it.
    // Checked by `ProjectHome` before anything deletes it: the id, the stored
    // folder and a link sitting where the folder goes all reach `remove_dir_all`.
    let home = match ProjectHome::of(store, root, project_id) {
        Ok(home) => Some(home),
        Err(HomeError::Forbidden) => None,
        Err(err) => return Err(home_refusal(err)),
    };
    let workspace = match &home {
        Some(home) => home.wipeable().map_err(home_refusal)?,
        None => None,
    };
    if !store.erase_project(project_id)? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such project"));
    }
    match (home, workspace) {
        (Some(home), Some(workspace)) => std::fs::remove_dir_all(&workspace)
            .map_err(|err| RpcError::internal(format!("{}: {err}", home.relative()))),
        (None, _) => {
            // No project on the notice: the row is gone, and would take it along.
            let title = "A forgotten project's folder was left on disk";
            let detail = "Its folder name is not one devpit gives, so nothing was deleted.";
            store.add_notice(None, FOLDER_NOTICE, title, Some(detail), None)?;
            Ok(())
        }
        (Some(_), None) => Ok(()),
    }
}

/// This project's folder in the devpit workspace, as the store names it.
pub(crate) fn project_home(project_id: &str) -> Result<ProjectHome, RpcError> {
    home_of(&store()?, &Store::root()?, project_id)
}

pub(crate) fn home_of(
    store: &Store,
    root: &Path,
    project_id: &str,
) -> Result<ProjectHome, RpcError> {
    ProjectHome::of(store, root, project_id).map_err(home_refusal)
}

pub(crate) fn home_refusal(err: HomeError) -> RpcError {
    let code = match err {
        HomeError::Invalid | HomeError::PluginId => ErrorCode::Invalid,
        HomeError::NotFound => ErrorCode::NotFound,
        HomeError::Forbidden | HomeError::Elsewhere | HomeError::PluginElsewhere => {
            ErrorCode::Forbidden
        }
        HomeError::Removal(_) | HomeError::Store(_) => ErrorCode::Internal,
    };
    RpcError::new(code, err.to_string())
}

/// `project.changes` — what has changed in a checkout, with the size of each edit.
#[tauri::command]
#[specta::specta]
pub async fn project_changes(
    project_id: String,
    worktree_id: Option<String>,
) -> Result<ProjectChanges, RpcError> {
    crate::off_main::blocking(move || project_changes_now(project_id, worktree_id)).await
}

/// [`project_changes`], on the calling thread.
pub(crate) fn project_changes_now(
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
    let output = devpit_pty::host_env::command("git")
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
#[path = "projects_tests.rs"]
mod tests;
