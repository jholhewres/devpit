//! The projects an orchestrator is linked to — chosen by the person, one by
//! one. Only those are on its Boards panel, readable by its chat, and within
//! reach of its tools; every other project stays where it is.
//!
//! Kept in the orchestrator's own settings file, beside its account, so the
//! link lives and goes with the orchestrator.

use std::path::Path;

use devpit_rpc::{ErrorCode, Project, RpcError};
use serde_json::{json, Value};

/// The settings as they are, or an empty object.
fn settings(folder: &Path) -> Value {
    std::fs::read_to_string(folder.join(devpit_core::home::ORCHESTRATOR_SETTINGS))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

/// One key of the settings changed, the rest kept.
pub(crate) fn set(folder: &Path, key: &str, value: Value) -> std::io::Result<()> {
    let file = folder.join(devpit_core::home::ORCHESTRATOR_SETTINGS);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut all = settings(folder);
    all[key] = value;
    std::fs::write(file, all.to_string())
}

/// The ids of the projects this orchestrator's folder is linked to.
pub(crate) fn linked(folder: &Path) -> Vec<String> {
    settings(folder)["projects"]
        .as_array()
        .map(|all| {
            all.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Whether `here`, an orchestrator, may reach `project`: itself, or one the
/// person linked. A project reaches only itself, and is not asked here.
pub(crate) fn reaches(here: &Project, project: &Project) -> Result<(), String> {
    if here.orchestrator.is_none() || project.id == here.id {
        return Ok(());
    }
    linked(Path::new(&here.root_path))
        .contains(&project.id)
        .then_some(())
        .ok_or_else(|| {
            format!(
                "{} is not linked to this orchestrator — ask the person to link it from the Boards panel",
                project.name
            )
        })
}

/// `orchestrator.links` — the projects this orchestrator is linked to.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_links(project_id: String) -> Result<Vec<String>, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        Ok(linked(&root))
    })
    .await
}

/// `orchestrator.link` — the projects it is linked to, as the person chose
/// them. Only projects, never another orchestrator.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_link(project_id: String, linked: Vec<String>) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        let projects = crate::projects::project_list_now()?.projects;
        if let Some(stray) = linked.iter().find(|id| {
            !projects
                .iter()
                .any(|one| &one.id == *id && one.orchestrator.is_none())
        }) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                format!("{stray} is not a project that can be linked"),
            ));
        }
        set(&root, "projects", json!(linked)).map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}

#[cfg(test)]
#[path = "orchestrator_links_tests.rs"]
mod tests;
