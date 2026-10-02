//! What a card's sessions wrote outside the card's own checkout.
//!
//! A card belongs to one project and its Changes read one checkout. Work
//! that reaches into another repository — a client and its API, an app and
//! its server — was invisible there: the checkout stayed clean while the
//! real diff sat somewhere else. The session's own hooks say every file it
//! edits or writes, so devpit keeps the ones outside the checkout, by
//! repository, and the card says "also changed 13 files in devpit-app".
//! Nothing about that other checkout is managed; it is only shown.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use devpit_rpc::{Elsewhere, ElsewhereFile, ErrorCode, Project, RpcError};
use serde::{Deserialize, Serialize};

/// The most files kept for one card, all repositories together.
const MOST_FILES: usize = 500;

/// What is kept: repository root → files, relative to it.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub(crate) struct Kept {
    pub repos: BTreeMap<String, Vec<String>>,
}

fn kept_file(project_id: &str, card_id: &str) -> Result<PathBuf, RpcError> {
    if card_id.contains(['/', '\\']) || card_id.starts_with('.') {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a card"));
    }
    Ok(crate::projects::project_home(project_id)?
        .elsewhere()
        .join(format!("{card_id}.json")))
}

fn read_kept(file: &Path) -> Kept {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn resolved(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The repository a file is in: the nearest folder above it holding `.git`.
pub(crate) fn repo_of(file: &Path) -> Option<PathBuf> {
    file.ancestors()
        .skip(1)
        .find(|dir| dir.join(".git").exists())
        .map(Path::to_path_buf)
}

/// `path`, written by a session of a card whose checkout is `home`, kept in
/// `kept` when it is outside it. Answers whether anything changed.
pub(crate) fn note(kept: &mut Kept, home: &Path, path: &Path) -> bool {
    if !path.is_absolute() {
        return false;
    }
    let file = resolved(path.parent().unwrap_or(path)).join(path.file_name().unwrap_or_default());
    if file.starts_with(resolved(home)) {
        return false;
    }
    let Some(repo) = repo_of(&file) else {
        return false;
    };
    let Ok(relative) = file.strip_prefix(&repo) else {
        return false;
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    let total: usize = kept.repos.values().map(Vec::len).sum();
    let files = kept.repos.entry(repo.display().to_string()).or_default();
    if files.contains(&relative) || total >= MOST_FILES {
        return false;
    }
    files.push(relative);
    true
}

/// A session of `card_id` edited or wrote `path`.
pub(crate) fn touched(card_id: &str, path: &str) {
    let Ok(store) = crate::projects::store() else {
        return;
    };
    let Some(project_id) = store.live_card_project(card_id).ok().flatten() else {
        return;
    };
    let Ok(Some(card)) = store.card(card_id) else {
        return;
    };
    let Ok(projects) = crate::live_sessions::recent_projects() else {
        return;
    };
    let Some(project) = projects.iter().find(|one| one.id == project_id) else {
        return;
    };
    let home = card
        .worktree_path
        .clone()
        .unwrap_or_else(|| project.root_path.clone());
    let Ok(file) = kept_file(&project_id, card_id) else {
        return;
    };
    let mut kept = read_kept(&file);
    if note(&mut kept, Path::new(&home), Path::new(path)) {
        if let Some(dir) = file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string(&kept) {
            let _ = std::fs::write(&file, text);
        }
    }
}

/// `card.elsewhere` — the other repositories this card's sessions wrote in.
#[tauri::command]
#[specta::specta]
pub async fn card_elsewhere(card_id: String) -> Result<Vec<Elsewhere>, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let Some(project_id) = store.live_card_project(&card_id)? else {
            return Ok(Vec::new());
        };
        let kept = read_kept(&kept_file(&project_id, &card_id)?);
        let projects = crate::live_sessions::recent_projects()?;
        Ok(shown(&kept, &projects))
    })
    .await
}

fn shown(kept: &Kept, projects: &[Project]) -> Vec<Elsewhere> {
    kept.repos
        .iter()
        .filter(|(_, files)| !files.is_empty())
        .map(|(root, files)| {
            let project = crate::agent_api::project_at(projects, Path::new(root));
            let dirty = devpit_git::status(Path::new(root))
                .map(|status| status.paths)
                .unwrap_or_default();
            Elsewhere {
                root: root.clone(),
                name: project.map(|one| one.name.clone()).unwrap_or_else(|| {
                    Path::new(root)
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| root.clone())
                }),
                project_id: project.map(|one| one.id.clone()),
                files: files
                    .iter()
                    .map(|path| ElsewhereFile {
                        path: path.clone(),
                        uncommitted: dirty.contains_key(path),
                    })
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "card_elsewhere_tests.rs"]
mod tests;
