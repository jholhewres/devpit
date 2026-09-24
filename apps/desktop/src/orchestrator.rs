//! `orchestrator.open` — the chat that sees every project, one per profile.
//!
//! An orchestrator is a project whose folder is devpit's own, under
//! `orchestrator/<profile>`. Being a project is what gives it the chat, the
//! tabs, the files and the MCP's scope for nothing; being known by its folder
//! is what spares the store a column that would say the same thing.

use std::path::Path;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, Project, RpcError};

/// What a new orchestrator's folder starts with. Written only when missing:
/// the person edits these, and a second opening must not undo that.
const SEEDED: &[(&str, &str)] = &[
    ("CLAUDE.md", include_str!("orchestrator_brief.md")),
    ("docs/.gitkeep", ""),
    ("artifacts/.gitkeep", ""),
    ("context/.gitkeep", ""),
];

/// `orchestrator.open` — this profile's orchestrator, made on first use.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_open(profile_id: String) -> Result<Project, RpcError> {
    crate::off_main::blocking(move || orchestrator_open_now(profile_id)).await
}

/// [`orchestrator_open`], on the calling thread.
pub(crate) fn orchestrator_open_now(profile_id: String) -> Result<Project, RpcError> {
    let store = crate::projects::store()?;
    let profile = crate::agent_profiles::all(&store)?
        .into_iter()
        .find(|one| one.id == profile_id)
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                format!("no profile called {profile_id}"),
            )
        })?;
    // Only Claude Code can reach its other sessions, which is the point.
    if profile.driver != "claude" {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "an orchestrator runs on Claude Code",
        ));
    }
    let root = Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    let folder = devpit_core::home::orchestrator_dir(&root, &profile.id).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Invalid,
            "that profile's name cannot name a folder",
        )
    })?;
    seed(&folder).map_err(|err| RpcError::internal(err.to_string()))?;
    devpit_git::init(&folder).map_err(|err| RpcError::internal(err.to_string()))?;

    let here = folder
        .canonicalize()
        .map_err(|err| RpcError::internal(err.to_string()))?;
    if let Some(row) = store
        .projects()?
        .into_iter()
        .find(|row| Path::new(&row.root_path) == here)
    {
        return Ok(crate::projects::drawn(&store, row));
    }
    let made = crate::projects::project_add_now(here.display().to_string())?;
    let named = crate::project_naming::project_edit_now(
        made.id.clone(),
        format!("Orchestrator · {}", profile.label),
        None,
        None,
        None,
    )?;
    Ok(named
        .projects
        .into_iter()
        .find(|one| one.id == made.id)
        .unwrap_or(made))
}

/// Writes what a new orchestrator starts with, leaving what is there alone.
pub(crate) fn seed(folder: &Path) -> std::io::Result<()> {
    for (name, text) in SEEDED {
        let path = folder.join(name);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "orchestrator_tests.rs"]
mod tests;
