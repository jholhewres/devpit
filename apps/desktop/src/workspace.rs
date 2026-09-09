//! What is actually on disk: the skills this machine has, and what the devpit
//! workspace is holding.
//!
//! Both panels drew fixed rows before this. A row that names a file and a size
//! it never read is worse than an empty panel, because an empty panel is
//! obviously empty.

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    /// Where it came from — `omc`, `claude`, `yours`.
    pub source: String,
    /// The `SKILL.md` itself, so Open and Reveal have something to hand over.
    pub path: String,
    /// The first line of prose in the file, when there is one.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Skills {
    pub skills: Vec<Skill>,
    /// Why nothing could be read, when nothing could. Present and non-empty
    /// means the panel says this instead of looking empty.
    pub problem: Option<String>,
}

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

/// The first line of a skill's prose, for the row's second line.
///
/// After the frontmatter, if there is any, and skipping the heading: the
/// heading repeats the name, and a row that says `tdd — # tdd` says nothing.
pub fn description_of(text: &str) -> String {
    let body = match text.strip_prefix("---\n") {
        Some(rest) => rest
            .split_once("\n---")
            .map(|(_, body)| body)
            .unwrap_or(rest),
        None => text,
    };
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("---"))
        .unwrap_or_default()
        .to_owned()
}

/// `skills.list` — the skills installed on this machine.
#[tauri::command]
#[specta::specta]
pub fn skills_list() -> Result<Skills, RpcError> {
    let sources = devpit_agentcli::skills::sources();
    if sources.is_empty() {
        return Ok(Skills {
            skills: Vec::new(),
            problem: Some("no skills directory on this machine".to_owned()),
        });
    }

    let mut skills = Vec::new();
    for dir in &sources {
        let source = devpit_agentcli::source_of(dir);
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let file = entry.path().join("SKILL.md");
            if !file.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if skills.iter().any(|had: &Skill| had.name == name) {
                continue;
            }
            skills.push(Skill {
                description: std::fs::read_to_string(&file)
                    .map(|text| description_of(&text))
                    .unwrap_or_default(),
                path: file.display().to_string(),
                name,
                source: source.clone(),
            });
        }
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Skills {
        problem: skills
            .is_empty()
            .then(|| "the skills directories are there but hold nothing".to_owned()),
        skills,
    })
}

/// What one entry of the workspace is, measured now.
fn measure(home: &Path, name: &str, relative: &str) -> Held {
    let path = home.join(relative);
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
        .map(|id| format!("projects/{id}"))
        .unwrap_or_default();

    let mut held = vec![
        measure(&home, "agents/", "agents"),
        measure(&home, "skills/", "skills"),
        measure(&home, "board.db", "board.db"),
        measure(&home, "hooks.json", "hooks.json"),
    ];
    if !mine.is_empty() {
        held.push(measure(&home, "sessions/", &format!("{mine}/sessions")));
        held.push(measure(&home, "prime.json", &format!("{mine}/prime.json")));
    }
    held.push(measure(&home, "worktrees/", "worktrees"));

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
