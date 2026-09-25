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
    tmux: Option<String>,
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
    screen: impl Fn(&str) -> Option<String>,
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
            let name = listed.name?;
            let project = crate::agent_api::project_at(projects, Path::new(&cwd));
            if project.is_some_and(|one| one.orchestrator.is_some()) {
                return None;
            }
            // The screen only of a session that is listed, and read once.
            let target = listed.tmux.as_deref().and_then(pane_target);
            let in_devpit = target.is_some();
            let waiting = target
                .and_then(|target| screen(&target))
                .and_then(|shown| crate::live_prompt::pending(&shown));
            Some(LiveSession {
                name,
                status: listed.status.unwrap_or_default(),
                kind: listed.kind.unwrap_or_default(),
                card_id: card_of(worktrees, Path::new(&cwd)),
                project_id: project.map(|one| one.id.clone()),
                project_name: project.map(|one| one.name.clone()),
                since: listed.status_updated_at,
                in_devpit,
                waiting,
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

/// Where to type into a session that runs in a devpit terminal: the CLI
/// records the tmux client it runs under, `devpit_<project>__<leaf>`, and the
/// leaf is the window. `None` for anything devpit did not name.
pub(crate) fn pane_target(listed: &str) -> Option<String> {
    // The CLI writes where it runs as `session:@window.%pane`; the session is
    // devpit's client, and the rest — tmux's own ids — says nothing more.
    let (client, at) = listed.split_once(':').unwrap_or((listed, ""));
    if !at
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '@' | '.' | '%'))
    {
        return None;
    }
    let (session, window) = client.split_once("__")?;
    let plain = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    };
    (session.starts_with("devpit_")
        && window.starts_with("leaf_")
        && plain(session)
        && plain(window))
    .then(|| format!("{client}:{window}"))
}

/// A devpit terminal's screen as it is now, or nothing when tmux cannot say.
pub(crate) fn screen_of(target: &str) -> Option<String> {
    crate::sessions::tmux_server()
        .ok()?
        .capture_pane(target)
        .ok()
}

/// The end of a session's screen and the question it is stopped on, for an
/// orchestrator to read — never to answer. `None` for a session that is not in
/// a devpit terminal.
pub(crate) fn screen_for(
    profile_id: &str,
    name: &str,
) -> Result<Option<(String, Option<devpit_rpc::PendingPrompt>)>, RpcError> {
    let client = client_named(profile_id, name)?;
    let Some(shown) = pane_target(&client).and_then(|target| screen_of(&target)) else {
        return Ok(None);
    };
    let lines: Vec<&str> = shown.lines().collect();
    // The bottom of the screen is where a session says what it is doing.
    let tail = lines[lines.len().saturating_sub(40)..]
        .join("\n")
        .trim_end()
        .to_owned();
    Ok(Some((tail, crate::live_prompt::pending(&shown))))
}

/// The longest reply typed for the person. A reply, not a document.
const LONGEST_REPLY: usize = 4000;

/// `orchestrator.reply` — types the person's own words into a session's
/// terminal, as if they had gone there and typed them.
///
/// The window's alone: no agent reaches it. A message from another session
/// approves nothing, by the CLI's rule; this is the person answering, and it
/// must stay only theirs to send.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_reply(
    profile_id: String,
    name: String,
    text: String,
) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > LONGEST_REPLY {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "a reply is between 1 and 4000 characters",
            ));
        }
        let target = terminal_of(&profile_id, &name)?;
        crate::sessions::tmux_server()?
            .paste_and_send(&target, text)
            .map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}

/// The devpit terminal a session of this account runs in, by its name: the
/// only place the window types or presses anything for the person.
pub(crate) fn terminal_of(profile_id: &str, name: &str) -> Result<String, RpcError> {
    let client = client_named(profile_id, name)?;
    pane_target(&client).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Invalid,
            format!("{name} is not in a devpit terminal"),
        )
    })
}

/// The tmux client of this account's session called `name`.
fn client_named(profile_id: &str, name: &str) -> Result<String, RpcError> {
    listed_clients(&config_of(profile_id)?.join("sessions"))
        .into_iter()
        .find(|(named, _)| named == name)
        .map(|(_, client)| client)
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                format!("no session called {name} is running"),
            )
        })
}

/// Each live listing's name and tmux client, for the one reply that needs it.
fn listed_clients(sessions: &Path) -> Vec<(String, String)> {
    let Ok(entries) = std::fs::read_dir(sessions) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() <= MOST_BYTES))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Listed>(&text).ok())
        .filter(|listed| listed.pid.is_some_and(alive))
        .filter_map(|listed| Some((listed.name?, listed.tmux?)))
        .collect()
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
    let projects = crate::projects::project_list_now()?.projects;
    let worktrees = devpit_core::Store::root()
        .ok()
        .and_then(|root| root.join("worktrees").canonicalize().ok())
        .unwrap_or_default();
    // One tmux server for every screen read in this listing.
    let server = crate::sessions::tmux_server().ok();
    let screen = |target: &str| server.as_ref()?.capture_pane(target).ok();
    Ok(LiveSessions {
        sessions: read(
            &config_of(profile_id)?.join("sessions"),
            alive,
            &projects,
            &worktrees,
            screen,
        ),
    })
}

/// This profile's account's config folder — where its sessions are listed.
/// Only its own: an orchestrator sees the sessions of the account it speaks
/// as, never another's.
fn config_of(profile_id: &str) -> Result<std::path::PathBuf, RpcError> {
    let store = crate::projects::store()?;
    crate::agent_profiles::all(&store)?
        .iter()
        .find(|one| one.id == profile_id)
        .map(config_dir)
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                format!("no profile called {profile_id}"),
            )
        })
}

/// Where a profile's Claude Code keeps its config — and so its sessions.
fn config_dir(profile: &devpit_rpc::Profile) -> std::path::PathBuf {
    let said = devpit_agentcli::running::runner(profile)
        .env
        .into_iter()
        .find(|(name, _)| name == "CLAUDE_CONFIG_DIR")
        .map(|(_, value)| value);
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    devpit_agentcli::cli_config::config_dir_from(&home, said.as_deref())
}

#[cfg(test)]
#[path = "live_sessions_tests.rs"]
mod tests;
