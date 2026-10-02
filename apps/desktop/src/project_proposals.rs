//! Changes to its projects an orchestrator proposes and the person makes.
//!
//! An orchestrator reaches only the projects it was linked to, so it must
//! not be the one to widen that: `devpit_propose_project` only proposes. The
//! window shows a card in the orchestrator's chat — add this folder, in this
//! group, and link it — and only the person's click does it, with the
//! commands the rest of the window already uses.
//!
//! Held in memory, a few per orchestrator: a proposal is for now.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use devpit_rpc::{Project, ProjectProposal};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

/// The most proposals one orchestrator keeps waiting; the oldest goes first.
const MOST: usize = 8;

type Waiting = HashMap<String, Vec<ProjectProposal>>;

fn waiting() -> &'static Mutex<Waiting> {
    static WAITING: OnceLock<Mutex<Waiting>> = OnceLock::new();
    WAITING.get_or_init(Mutex::default)
}

/// What one call to `devpit_propose_project` asks for.
pub(crate) struct Asked<'a> {
    pub path: Option<&'a str>,
    pub project: Option<&'a str>,
    pub name: Option<&'a str>,
    pub group: Option<&'a str>,
    pub link: Option<bool>,
}

/// The proposal `asked` makes from the orchestrator `here`, among
/// `projects`, with `linked` the ones it reaches now. Kept for the window,
/// which `app` tells.
pub(crate) fn propose(
    app: Option<&AppHandle>,
    here: &Project,
    projects: &[Project],
    linked: &[String],
    home: Option<&Path>,
    asked: &Asked,
) -> Result<Value, String> {
    let proposal = proposal_of(projects, linked, home, asked)?;
    let id = proposal.id.clone();
    {
        let mut held = waiting()
            .lock()
            .map_err(|_| "the proposals could not be read")?;
        let list = held.entry(here.id.clone()).or_default();
        // The same folder proposed again replaces what was proposed for it.
        list.retain(|one| one.path != proposal.path);
        list.push(proposal);
        if list.len() > MOST {
            list.remove(0);
        }
    }
    if let Some(app) = app {
        let _ = app.emit("orchestrator:proposed", &here.id);
    }
    Ok(json!({
        "proposed": id,
        "note": "Nothing was changed. The person sees a card in this chat to do it, edit it first, or drop it.",
    }))
}

pub(crate) fn proposal_of(
    projects: &[Project],
    linked: &[String],
    home: Option<&Path>,
    asked: &Asked,
) -> Result<ProjectProposal, String> {
    let words = |text: Option<&str>| {
        text.map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    let name = words(asked.name);
    let group = words(asked.group);
    if name.as_ref().is_some_and(|name| name.chars().count() > 80)
        || group
            .as_ref()
            .is_some_and(|group| group.chars().count() > 80)
    {
        return Err("a name or a group is at most 80 characters".to_owned());
    }
    let existing = match (asked.project, asked.path) {
        (Some(named), _) => Some(
            projects
                .iter()
                .find(|one| one.orchestrator.is_none() && (one.id == named || one.name == named))
                .ok_or_else(|| {
                    format!("no project is called {named} — pass a folder's path to add one")
                })?,
        ),
        (None, Some(path)) => {
            let folder = folder_of(path, home)?;
            projects
                .iter()
                .find(|one| one.orchestrator.is_none() && same(Path::new(&one.root_path), &folder))
        }
        (None, None) => return Err("pass the folder's path, or the project".to_owned()),
    };
    match existing {
        Some(project) => {
            let is_linked = linked.contains(&project.id);
            let link = asked
                .link
                .or((!is_linked).then_some(true))
                .filter(|link| *link != is_linked);
            let rename = name.filter(|name| *name != project.name);
            let regroup = group.filter(|group| Some(group) != project.group.as_ref());
            if link.is_none() && rename.is_none() && regroup.is_none() {
                return Err(format!(
                    "{} is already as proposed — nothing to change",
                    project.name
                ));
            }
            Ok(ProjectProposal {
                id: format!("prop_{}", ulid::Ulid::generate()),
                project_id: Some(project.id.clone()),
                path: project.root_path.clone(),
                name: rename.unwrap_or_else(|| project.name.clone()),
                group: regroup,
                link,
            })
        }
        None => {
            let path = asked.path.ok_or("pass the folder's path")?;
            let folder = folder_of(path, home)?;
            let named = folder
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(ProjectProposal {
                id: format!("prop_{}", ulid::Ulid::generate()),
                project_id: None,
                path: folder.display().to_string(),
                name: name.unwrap_or(named),
                group,
                link: Some(asked.link.unwrap_or(true)),
            })
        }
    }
}

/// A folder that is there, whole: `~/` read as the home, nothing relative.
fn folder_of(path: &str, home: Option<&Path>) -> Result<PathBuf, String> {
    let path = path.trim();
    let whole = match path.strip_prefix("~/") {
        Some(rest) => home.ok_or("the home folder is unknown")?.join(rest),
        None => PathBuf::from(path),
    };
    if !whole.is_absolute() {
        return Err("pass the folder's whole path, or one from ~/".to_owned());
    }
    let found = std::fs::canonicalize(&whole)
        .map_err(|_| format!("there is no folder at {}", whole.display()))?;
    if !found.is_dir() {
        return Err(format!("{} is not a folder", found.display()));
    }
    Ok(found)
}

fn same(one: &Path, other: &Path) -> bool {
    std::fs::canonicalize(one).unwrap_or_else(|_| one.to_path_buf()) == other
}

/// `orchestrator.proposals` — what this orchestrator proposed, oldest first.
#[tauri::command]
#[specta::specta]
pub fn orchestrator_proposals(project_id: String) -> Vec<ProjectProposal> {
    waiting()
        .lock()
        .ok()
        .and_then(|held| held.get(&project_id).cloned())
        .unwrap_or_default()
}

/// `orchestrator.proposal_drop` — done or dropped by the person, it goes.
#[tauri::command]
#[specta::specta]
pub fn orchestrator_proposal_drop(project_id: String, id: String) {
    if let Ok(mut held) = waiting().lock() {
        if let Some(list) = held.get_mut(&project_id) {
            list.retain(|one| one.id != id);
        }
    }
}

#[cfg(test)]
#[path = "project_proposals_tests.rs"]
mod tests;
