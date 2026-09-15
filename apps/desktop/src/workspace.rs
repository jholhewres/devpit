//! What the devpit workspace is holding, measured.
//!
//! The panel drew fixed rows before this. A row that names a file and a size
//! it never read is worse than an empty panel, because an empty panel is
//! obviously empty.
//!
//! The skills this machine has moved to `skills.rs`: "how much room is this
//! taking" and "what does this machine know how to do" are different
//! questions, and they were sharing a file only because both were new.

use std::path::PathBuf;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// One thing the workspace holds, measured.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Held {
    pub name: String,
    pub path: String,
    /// Bytes, measured now.
    pub bytes: f64,
    /// How many things are in it, for a directory. Absent for a file.
    pub count: Option<u32>,
    pub is_dir: bool,
    /// False when the workspace has not made it yet — a row that says "not
    /// yet" is honest; a row that says 0 B is not.
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub directory: String,
    pub held: Vec<Held>,
    /// Everything above, added up. Worktrees and transcripts grow without
    /// announcing themselves.
    pub bytes: f64,
}

/// What one entry of the workspace is, measured now.
fn measure(name: &str, path: PathBuf) -> Held {
    let is_dir = path.is_dir();
    Held {
        name: name.to_owned(),
        bytes: if is_dir {
            devpit_git::disk_usage(&path) as f64
        } else {
            std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0) as f64
        },
        count: is_dir.then(|| {
            std::fs::read_dir(&path)
                .map(|entries| entries.flatten().count() as u32)
                .unwrap_or(0)
        }),
        exists: path.exists(),
        path: path.display().to_string(),
        is_dir,
    }
}

/// `workspace.read` — the devpit workspace, row by row, with real sizes.
#[tauri::command]
#[specta::specta]
pub fn workspace_read(project_id: Option<String>) -> Result<Workspace, RpcError> {
    let home: PathBuf =
        Store::root().map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?;
    let mine = project_id
        .as_deref()
        .map(crate::projects::project_home)
        .transpose()?;

    let mut held = vec![
        measure("agents/", home.join("agents")),
        measure("skills/", home.join("skills")),
        measure("board.db", home.join("board.db")),
        measure("hooks.json", home.join("hooks.json")),
    ];
    if let Some(mine) = &mine {
        held.push(measure("sessions/", mine.sessions()));
        held.push(measure("prime.json", mine.prime()));
    }
    held.push(measure("worktrees/", home.join("worktrees")));

    Ok(Workspace {
        directory: home.display().to_string(),
        bytes: held.iter().map(|one| one.bytes).sum(),
        held,
    })
}

/// What a project has actually spent.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Spend {
    pub usd: f64,
    /// How many runs reported a cost. Zero means nothing has been measured,
    /// which the screen says rather than drawing a zero.
    pub runs: u32,
    /// Title and cost, most expensive first.
    pub cards: Vec<(String, f64)>,
}

/// `usage.read` — what this project has spent, measured.
///
/// Only what the runs recorded. There is no daily series and no per-model
/// split because nothing records either yet, and a chart of numbers nobody
/// measured is the thing this milestone exists to delete.
#[tauri::command]
#[specta::specta]
pub fn usage_read(project_id: String) -> Result<Spend, RpcError> {
    let store = Store::open_default()?;
    let (usd, runs) = store.project_spend(&project_id)?;
    Ok(Spend {
        usd,
        runs,
        cards: store.dearest_cards(&project_id, 10)?,
    })
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
