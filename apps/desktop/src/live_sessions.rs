//! The sessions an account has running, read where the CLI lists them.
//!
//! Claude Code writes one small file per live session under its configuration
//! folder: the name other sessions message it by, whether it is busy, the
//! folder it runs in. That is the list an orchestrator can reach, so it is the
//! list it is shown — with the project and card each one works in.

use std::collections::HashMap;
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
    /// The short id a background session is stopped by.
    job_id: Option<String>,
    session_id: Option<String>,
    /// When the process started, in the kernel's clock ticks since boot.
    proc_start: Option<String>,
}

impl Listed {
    /// Whether the listing's process is the one running now. A pid is reused
    /// after a restart, and a listing the CLI never removed then names
    /// whatever process took the number.
    fn running(&self) -> bool {
        self.pid
            .is_some_and(|pid| alive(pid) && same_start(pid, self.proc_start.as_deref()))
    }
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
    windows: &HashMap<String, String>,
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
        .filter(|listed| {
            listed
                .pid
                .is_some_and(|pid| alive(pid) && same_start(pid, listed.proc_start.as_deref()))
        })
        .filter_map(|listed| {
            let cwd = listed.cwd?;
            let name = listed.name?;
            // A card's checkout is under devpit's own folder, not its project's.
            let project = crate::agent_api::project_at(projects, Path::new(&cwd)).or_else(|| {
                let id = project_of_checkout(worktrees, Path::new(&cwd))?;
                projects.iter().find(|one| one.id == id)
            });
            if project.is_some_and(|one| one.orchestrator.is_some()) {
                return None;
            }
            // The screen only of a session that is listed, and read once.
            let target = listed
                .tmux
                .as_deref()
                .and_then(|tmux| pane_target(tmux, windows));
            let in_devpit = target.is_some();
            let pane = target.as_deref().and_then(pane_of);
            let waiting = target
                .and_then(|target| screen(&target))
                .and_then(|shown| crate::live_prompt::pending(&shown));
            let job = listed
                .job_id
                .filter(|_| listed.kind.as_deref() == Some("bg"));
            Some(LiveSession {
                name,
                pid: listed.pid.unwrap_or_default(),
                job,
                status: listed.status.unwrap_or_default(),
                kind: listed.kind.unwrap_or_default(),
                card_id: card_of(worktrees, Path::new(&cwd)),
                project_id: project.map(|one| one.id.clone()),
                project_name: project.map(|one| one.name.clone()),
                since: listed.status_updated_at,
                in_devpit,
                waiting,
                pane,
                cwd,
                draft: None,
                step: listed
                    .session_id
                    .as_deref()
                    .and_then(crate::island_feed::step_of),
                session_id: listed.session_id.filter(|id| crate::adopting::plain(id)),
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

/// The project whose card checkout `cwd` is in: `worktrees/<project>/<card>`.
pub(crate) fn project_of_checkout(worktrees: &Path, cwd: &Path) -> Option<String> {
    card_of(worktrees, cwd)?;
    let rest = cwd.strip_prefix(worktrees).ok()?;
    Some(rest.components().next()?.as_os_str().to_str()?.to_owned())
}

/// Where to type into a session that runs in a devpit terminal, as
/// `devpit_<project>:<leaf>`. `None` for anything devpit did not name, and for
/// a pane tmux no longer has.
///
/// The CLI writes where it runs once, as `session:@window.%pane`. The session
/// is the name tmux resolved for the pane at that moment, and with grouped
/// client sessions that can be another pane's client: every one of them shows
/// every window. So the pane id decides, looked up in `windows` (pane id →
/// window, as tmux has it now); the session's name only says the project.
pub(crate) fn pane_target(listed: &str, windows: &HashMap<String, String>) -> Option<String> {
    let (client, at) = listed.split_once(':').unwrap_or((listed, ""));
    if !at
        .chars()
        .all(|ch| ch.is_ascii_digit() || matches!(ch, '@' | '.' | '%'))
    {
        return None;
    }
    let plain = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    };
    let (session, named) = match client.split_once("__") {
        Some((session, window)) => (session, Some(window)),
        None => (client, None),
    };
    if !session.starts_with("devpit_") || !plain(session) {
        return None;
    }
    let window = match at.split_once('%') {
        Some((_, pane)) if !pane.is_empty() => windows.get(&format!("%{pane}"))?.as_str(),
        // An older CLI wrote the session alone, and the client's name is all
        // there is to go on.
        _ => named?,
    };
    // The project's own session holds every window of the group, so the
    // target never depends on which client sessions happen to exist.
    (window.starts_with("leaf_") && plain(window)).then(|| format!("{session}:{window}"))
}

/// The project and pane a devpit terminal's target names:
/// `devpit_<project>:<leaf>`.
pub(crate) fn pane_of(target: &str) -> Option<devpit_rpc::LivePane> {
    let (session, window) = target.split_once(':')?;
    // A client's target, `devpit_<project>__<leaf>:<leaf>`, names the same pane.
    let session = session
        .split_once("__")
        .map_or(session, |(project, _)| project);
    Some(devpit_rpc::LivePane {
        project_id: session.strip_prefix("devpit_")?.to_owned(),
        pane_id: window.to_owned(),
    })
}

/// Which window each of devpit's panes is in now. Empty when tmux cannot say,
/// and then no session is placed in a terminal rather than in a wrong one.
pub(crate) fn windows_now() -> HashMap<String, String> {
    crate::sessions::tmux_server()
        .ok()
        .and_then(|server| server.pane_windows().ok())
        .unwrap_or_default()
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
    let Some(target) = pane_target(&client, &windows_now()) else {
        return Ok(None);
    };
    let Some(shown) = screen_of(&target) else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("{name}'s terminal could not be read"),
        ));
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
    crate::off_main::blocking(move || reply_now(&profile_id, &name, &text)).await
}

/// [`orchestrator_reply`], on the calling thread: the Remote sends a draft through it too.
pub(crate) fn reply_now(profile_id: &str, name: &str, text: &str) -> Result<(), RpcError> {
    let text = text.trim();
    if text.is_empty() || text.chars().count() > LONGEST_REPLY {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "a reply is between 1 and 4000 characters",
        ));
    }
    let target = terminal_of(profile_id, name)?;
    let tmux = crate::sessions::tmux_server()?;
    // A session suspended or exited leaves its shell in front, and the
    // reply would run there as a command.
    if tmux
        .shell_in_front(&target)
        .map_err(|err| RpcError::internal(err.to_string()))?
    {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "the agent is not in front in that terminal — open it to see why",
        ));
    }
    tmux.paste_and_send(&target, text)
        .map_err(|err| RpcError::internal(err.to_string()))?;
    // Sent as the person: whatever was drafted for it has been said.
    crate::reply_drafts::forget(profile_id, name);
    Ok(())
}

/// The devpit terminal a session of this account runs in, by its name: the
/// only place the window types or presses anything for the person.
pub(crate) fn terminal_of(profile_id: &str, name: &str) -> Result<String, RpcError> {
    let client = match client_named(profile_id, name) {
        Ok(client) => client,
        // One started here that the CLI does not list yet: its terminal is
        // the one devpit typed it into.
        Err(err) => return crate::starting::target_of(profile_id, name).ok_or(err),
    };
    pane_target(&client, &windows_now()).ok_or_else(|| {
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

/// One live session of an account, as stopping it needs it.
pub(crate) struct Running {
    pub pid: i32,
    /// When it runs in one of devpit's terminals, that terminal.
    pub pane: Option<devpit_rpc::LivePane>,
    /// When it is a background session, the id its CLI stops it by.
    pub job: Option<String>,
}

/// Every live session of this account called `name`. More than one is
/// ordinary: a background job started twice keeps the name it was given.
pub(crate) fn running_named(profile_id: &str, name: &str) -> Result<Vec<Running>, RpcError> {
    let sessions = config_of(profile_id)?.join("sessions");
    let windows = windows_now();
    let found: Vec<Running> = std::fs::read_dir(&sessions)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "json"))
        .filter(|entry| entry.metadata().is_ok_and(|meta| meta.len() <= MOST_BYTES))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Listed>(&text).ok())
        .filter(|listed| listed.name.as_deref() == Some(name) && listed.running())
        .map(|listed| Running {
            pid: listed.pid.unwrap_or_default(),
            pane: listed
                .tmux
                .as_deref()
                .and_then(|tmux| pane_target(tmux, &windows))
                .as_deref()
                .and_then(pane_of),
            job: listed
                .job_id
                .filter(|_| listed.kind.as_deref() == Some("bg")),
        })
        .collect();
    if found.is_empty() {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("no session called {name} is running"),
        ));
    }
    Ok(found)
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
        .filter(Listed::running)
        .filter_map(|listed| Some((listed.name?, listed.tmux?)))
        .collect()
}

/// Whether a process is still there. A listing outlives a CLI that crashed.
pub(crate) fn alive(pid: i32) -> bool {
    devpit_pty::process::alive(pid)
}

/// Whether `pid` started when the listing says. Only Linux keeps the start in
/// `/proc`; elsewhere, and for a listing that does not say, it is taken as so.
fn same_start(pid: i32, listed: Option<&str>) -> bool {
    let Some(listed) = listed else {
        return true;
    };
    match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => start_of(&stat).is_none_or(|now| now == listed),
        Err(_) => true,
    }
}

/// The start time in a `/proc/<pid>/stat` line: its 22nd field. The second is
/// the command in parentheses, which may hold spaces, so counting starts after
/// the last `)`.
pub(crate) fn start_of(stat: &str) -> Option<&str> {
    let (_, rest) = stat.rsplit_once(')')?;
    rest.split_whitespace().nth(19)
}

/// How long the project list is reused between listings. Drawing it asks git
/// about every project's worktrees, and the sessions are read every few
/// seconds while an orchestrator is on screen — a git per project, each time.
const PROJECTS_FRESH: std::time::Duration = std::time::Duration::from_secs(15);

static PROJECTS: std::sync::Mutex<Option<(std::time::Instant, Vec<Project>)>> =
    std::sync::Mutex::new(None);

pub(crate) fn recent_projects() -> Result<Vec<Project>, RpcError> {
    // Reused only while it lists the projects there are: one added a moment
    // ago left its sessions with no project, so out of an orchestrator's reach.
    let mut there: Vec<String> = crate::projects::store()?
        .projects()?
        .into_iter()
        .map(|row| row.id)
        .collect();
    there.sort();
    if let Some((at, kept)) = PROJECTS.lock().ok().and_then(|held| held.clone()) {
        if at.elapsed() < PROJECTS_FRESH && same_ids(kept.iter().map(|one| one.id.as_str()), &there)
        {
            return Ok(kept);
        }
    }
    let fresh = crate::projects::project_list_now()?.projects;
    if let Ok(mut held) = PROJECTS.lock() {
        *held = Some((std::time::Instant::now(), fresh.clone()));
    }
    Ok(fresh)
}

/// Whether a kept list is of the projects `there` names, in any order.
pub(crate) fn same_ids<'a>(kept: impl Iterator<Item = &'a str>, there: &[String]) -> bool {
    let mut kept: Vec<&str> = kept.collect();
    kept.sort_unstable();
    kept.len() == there.len() && kept.iter().zip(there).all(|(one, other)| *one == other)
}

/// `orchestrator.sessions` — what this profile's account has running now.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_sessions(profile_id: String) -> Result<LiveSessions, RpcError> {
    crate::off_main::blocking(move || orchestrator_sessions_now(&profile_id)).await
}

/// [`orchestrator_sessions`], on the calling thread.
pub(crate) fn orchestrator_sessions_now(profile_id: &str) -> Result<LiveSessions, RpcError> {
    let projects = recent_projects()?;
    let worktrees = devpit_core::Store::root()
        .ok()
        .and_then(|root| root.join("worktrees").canonicalize().ok())
        .unwrap_or_default();
    // One tmux server for every screen read in this listing.
    let server = crate::sessions::tmux_server().ok();
    let screen = |target: &str| server.as_ref()?.capture_pane(target).ok();
    let windows = server
        .as_ref()
        .and_then(|server| server.pane_windows().ok())
        .unwrap_or_default();
    let mut sessions = read(
        &config_of(profile_id)?.join("sessions"),
        alive,
        &projects,
        &worktrees,
        &windows,
        screen,
    );
    // Started here and stopped before the CLI lists it — on the folder's
    // trust question, most often.
    let early = crate::starting::unlisted(profile_id, &sessions, screen);
    sessions.extend(early);
    for one in &mut sessions {
        one.draft = crate::reply_drafts::drafted(profile_id, &one.name);
    }
    Ok(LiveSessions { sessions })
}

/// This profile's account's config folder — where its sessions are listed.
/// Only its own: an orchestrator sees the sessions of the account it speaks
/// as, never another's.
pub(crate) fn config_of(profile_id: &str) -> Result<std::path::PathBuf, RpcError> {
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
