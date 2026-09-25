//! A project's artifacts: files kept for it outside its repository.
//!
//! They live in the project's own folder in the devpit workspace, so a clone,
//! a new worktree or a `git clean` does not take them, and nothing commits
//! them by accident. A session keeps a file there and takes one back through
//! devpit's tools; the window lists them in the right panel.
//!
//! Every path an agent names is held to the project: its checkout or one of
//! its worktrees on one side, the artifacts folder on the other. An artifact's
//! name is a plain relative path — no `..`, nothing absolute.

use std::path::{Component, Path, PathBuf};

use devpit_rpc::{ErrorCode, Project, RpcError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use specta::Type;

/// How many are listed at most: a list, not an archive browser.
const MOST: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    /// Relative to the artifacts folder.
    pub name: String,
    pub bytes: f64,
    /// Milliseconds since the epoch.
    pub modified: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Artifacts {
    /// Where they are, whole: to open or reveal.
    pub folder: String,
    pub items: Vec<Artifact>,
}

/// `artifacts.list` — what the project keeps outside its repository.
#[tauri::command]
#[specta::specta]
pub async fn artifacts_list(project_id: String) -> Result<Artifacts, RpcError> {
    crate::off_main::blocking(move || {
        let folder = folder_of(&project_id)?;
        Ok(Artifacts {
            items: listed(&folder),
            folder: folder.display().to_string(),
        })
    })
    .await
}

/// `artifacts.remove` — one of them, gone.
#[tauri::command]
#[specta::specta]
pub async fn artifact_remove(project_id: String, name: String) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let folder = folder_of(&project_id)?;
        remove(&folder, &name).map_err(|why| RpcError::new(ErrorCode::Invalid, why))
    })
    .await
}

/// An agent's call: `artifacts`, `artifact_save`, `artifact_restore` or
/// `artifact_remove`, for `project`, from an agent standing in `cwd`.
pub(crate) fn respond(
    method: &str,
    project: &Project,
    cwd: &Path,
    params: &Value,
) -> Result<Value, String> {
    let folder = folder_of(&project.id).map_err(|err| err.message)?;
    let text = |name: &str| params.get(name).and_then(Value::as_str).unwrap_or_default();
    let flag = |name: &str| params.get(name).and_then(Value::as_bool).unwrap_or(false);
    let roots = roots_of(project);
    match method {
        "artifacts" => {
            Ok(json!({ "folder": folder.display().to_string(), "items": listed(&folder) }))
        }
        "artifact_save" => {
            let from = inside(&roots, cwd, text("from"))?;
            let name = if text("name").is_empty() {
                from.file_name()
                    .and_then(|one| one.to_str())
                    .unwrap_or_default()
                    .to_owned()
            } else {
                text("name").to_owned()
            };
            let to = folder.join(plain(&name)?);
            carry(&from, &to, flag("move"), flag("replace"))?;
            Ok(json!({ "saved": name, "path": to.display().to_string() }))
        }
        "artifact_restore" => {
            let from = folder.join(plain(text("name"))?);
            let to = inside_new(&roots, cwd, text("to"))?;
            carry(&from, &to, flag("move"), flag("replace"))?;
            Ok(json!({ "restored": to.display().to_string() }))
        }
        "artifact_remove" => {
            remove(&folder, text("name")).map(|()| json!({ "removed": text("name") }))
        }
        _ => Err(format!("devpit does not answer `{method}`")),
    }
}

fn folder_of(project_id: &str) -> Result<PathBuf, RpcError> {
    Ok(crate::projects::project_home(project_id)?.artifacts())
}

/// The folders an agent's files may come from or go to: the project's
/// checkout and its worktrees.
fn roots_of(project: &Project) -> Vec<PathBuf> {
    std::iter::once(project.root_path.as_str())
        .chain(project.worktrees.iter().map(|one| one.path.as_str()))
        .filter_map(|one| Path::new(one).canonicalize().ok())
        .collect()
}

/// An artifact's name as a relative path, or why it is not one.
pub(crate) fn plain(name: &str) -> Result<PathBuf, String> {
    let path = Path::new(name.trim());
    let fine = !name.trim().is_empty()
        && path
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
    fine.then(|| path.to_path_buf())
        .ok_or_else(|| format!("`{name}` is not a plain name: a relative path, without `..`"))
}

/// An existing file, named relative to `cwd` or whole, inside one of `roots`.
pub(crate) fn inside(roots: &[PathBuf], cwd: &Path, named: &str) -> Result<PathBuf, String> {
    let whole = cwd.join(named.trim());
    let real = whole
        .canonicalize()
        .map_err(|_| format!("there is no file at {}", whole.display()))?;
    held(roots, &real)?;
    real.is_file()
        .then_some(real)
        .ok_or_else(|| format!("{named} is not a file"))
}

/// A place a file may be written to: its folder exists inside one of `roots`.
pub(crate) fn inside_new(roots: &[PathBuf], cwd: &Path, named: &str) -> Result<PathBuf, String> {
    if named.trim().is_empty() {
        return Err("say where it goes: `to`, a path in the project".to_owned());
    }
    let whole = cwd.join(named.trim());
    let (Some(parent), Some(file)) = (whole.parent(), whole.file_name()) else {
        return Err(format!("{named} is not a file's path"));
    };
    let parent = parent
        .canonicalize()
        .map_err(|_| format!("the folder {} does not exist", parent.display()))?;
    held(roots, &parent)?;
    Ok(parent.join(file))
}

fn held(roots: &[PathBuf], real: &Path) -> Result<(), String> {
    roots
        .iter()
        .any(|root| real.starts_with(root))
        .then_some(())
        .ok_or_else(|| format!("{} is outside the project", real.display()))
}

/// Copies, or moves, a file; refuses to write over one unless told to.
fn carry(from: &Path, to: &Path, moving: bool, replace: bool) -> Result<(), String> {
    if !from.is_file() {
        return Err(format!("there is no file at {}", from.display()));
    }
    if to.exists() && !replace {
        return Err(format!(
            "{} is already there — pass replace: true to write over it",
            to.display()
        ));
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    std::fs::copy(from, to).map_err(|err| err.to_string())?;
    if moving {
        std::fs::remove_file(from).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn remove(folder: &Path, name: &str) -> Result<(), String> {
    let path = folder.join(plain(name)?);
    std::fs::remove_file(&path).map_err(|_| format!("there is no artifact called {name}"))?;
    // The folders a nested name made go with their last file.
    let mut dir = path.parent();
    while let Some(one) = dir.filter(|one| *one != folder) {
        if std::fs::remove_dir(one).is_err() {
            break;
        }
        dir = one.parent();
    }
    Ok(())
}

/// Every file in the folder, by name.
pub(crate) fn listed(folder: &Path) -> Vec<Artifact> {
    let mut found = Vec::new();
    let mut pending = vec![folder.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                pending.push(path);
            } else if let Ok(name) = path.strip_prefix(folder) {
                found.push(Artifact {
                    name: name.display().to_string(),
                    bytes: meta.len() as f64,
                    modified: meta
                        .modified()
                        .ok()
                        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|at| at.as_millis() as f64),
                });
            }
        }
    }
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found.truncate(MOST);
    found
}

#[cfg(test)]
#[path = "artifacts_tests.rs"]
mod tests;
