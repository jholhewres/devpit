//! The sessions an account has running, read where the CLI lists them.
//!
//! Claude Code writes one small file per live session under its configuration
//! folder: the name other sessions message it by, whether it is busy, the
//! folder it runs in. That is the list an orchestrator can reach, so it is the
//! list it is shown — with the project and card each one works in.

use std::path::Path;

use devpit_rpc::{ErrorCode, LiveSession, LiveSessions, Project, RpcError};
use serde::Deserialize;

/// A listing is a few hundred bytes; anything past this is not one.
const MOST_BYTES: u64 = 64 * 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Listed {
    pid: Option<i32>,
    name: Option<String>,
    status: Option<String>,
    kind: Option<String>,
    cwd: Option<String>,
    status_updated_at: Option<f64>,
}

/// Every live session listed in `sessions`, placed on its project and card.
///
/// A session running in an orchestrator is left out: it is the orchestrator
/// talking, not work it could follow.
pub(crate) fn read(
    sessions: &Path,
    alive: impl Fn(i32) -> bool,
    projects: &[Project],
    worktrees: &Path,
) -> Vec<LiveSession> {
    let Ok(entries) = std::fs::read_dir(sessions) else {
        return Vec::new();
    };
    let mut found: Vec<LiveSession> = entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() <= MOST_BYTES))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Listed>(&text).ok())
        .filter(|listed| listed.pid.is_some_and(&alive))
        .filter_map(|listed| {
            let cwd = listed.cwd?;
            let project = crate::agent_api::project_at(projects, Path::new(&cwd));
            if project.is_some_and(|one| one.orchestrator.is_some()) {
                return None;
            }
            Some(LiveSession {
                name: listed.name?,
                status: listed.status.unwrap_or_default(),
                kind: listed.kind.unwrap_or_default(),
                card_id: card_of(worktrees, Path::new(&cwd)),
                project_id: project.map(|one| one.id.clone()),
                project_name: project.map(|one| one.name.clone()),
                since: listed.status_updated_at,
                cwd,
            })
        })
        .collect();
    found.sort_by(|a, b| a.name.cmp(&b.name));
    found
}

/// The card whose checkout `cwd` is in: devpit makes them at
/// `worktrees/<project>/<card>`.
pub(crate) fn card_of(worktrees: &Path, cwd: &Path) -> Option<String> {
    let rest = cwd.strip_prefix(worktrees).ok()?;
    let card = rest.components().nth(1)?.as_os_str().to_str()?;
    card.starts_with("card_").then(|| card.to_owned())
}

/// Whether a process is still there. A listing outlives a CLI that crashed.
pub(crate) fn alive(pid: i32) -> bool {
    // Signal 0 checks without sending anything. EPERM is a process that
    // exists and belongs to someone else, which is still alive.
    pid > 0
        && (unsafe { libc::kill(pid, 0) } == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM))
}

/// `orchestrator.sessions` — what this profile's account has running now.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_sessions(profile_id: String) -> Result<LiveSessions, RpcError> {
    crate::off_main::blocking(move || orchestrator_sessions_now(&profile_id)).await
}

/// [`orchestrator_sessions`], on the calling thread.
pub(crate) fn orchestrator_sessions_now(profile_id: &str) -> Result<LiveSessions, RpcError> {
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
    let said = devpit_agentcli::running::runner(&profile)
        .env
        .into_iter()
        .find(|(name, _)| name == "CLAUDE_CONFIG_DIR")
        .map(|(_, value)| value);
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let config = devpit_agentcli::cli_config::config_dir_from(&home, said.as_deref());
    let projects = crate::projects::project_list_now()?.projects;
    let worktrees = devpit_core::Store::root()
        .ok()
        .and_then(|root| root.join("worktrees").canonicalize().ok())
        .unwrap_or_default();
    Ok(LiveSessions {
        sessions: read(&config.join("sessions"), alive, &projects, &worktrees),
    })
}

#[cfg(test)]
#[path = "live_sessions_tests.rs"]
mod tests;
