//! Sessions devpit has just typed into a terminal, before the CLI lists them.
//!
//! Claude Code writes a session into its registry only once it is up — after
//! the person told it to trust the folder, when the folder is new to it. Until
//! then an orchestrator that started one saw nothing: not the session, not the
//! question it was stopped on, and the person learned of it by finding the
//! tab. devpit opened that terminal, so it reads it from the first second, and
//! a question there is shown and answered like any other.

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use devpit_rpc::{LivePane, LiveSession};
use serde_json::{json, Value};

/// How long a session that never came up is still looked for.
const LOOKED_FOR: Duration = Duration::from_secs(10 * 60);

/// How long a start waits for its session to be reachable before answering.
pub(crate) const UP_WITHIN: Duration = Duration::from_secs(30);
const LOOKED_EVERY: Duration = Duration::from_millis(250);

const UP: &str = "It runs in a terminal tab of the project, where the person can watch it, and is up: message it by this name with SendMessage, and pass notify_when_idle to hear when it is done.";
const NOT_UP: &str = "It runs in a terminal tab of the project but was not up in time, most often stopped on a question there. A SendMessage before it is up is lost: message it only once devpit_sessions lists it.";

#[derive(Clone)]
pub(crate) struct Starting {
    pub profile: String,
    pub name: String,
    pub project_id: String,
    pub project_name: Option<String>,
    pub cwd: String,
    pub pane_id: String,
    pub at: Instant,
}

impl Starting {
    /// Its pane's target, tmux's or the daemon's; tmux's spelling for a
    /// leaf no stored layout holds.
    fn target(&self) -> String {
        crate::pane_io::target_of(&self.project_id, &self.pane_id).unwrap_or_else(|_| {
            devpit_tmux::Server::target(
                &devpit_tmux::Server::session_name(&self.project_id),
                &self.pane_id,
            )
        })
    }
}

fn starting() -> &'static Mutex<Vec<Starting>> {
    static STARTING: OnceLock<Mutex<Vec<Starting>>> = OnceLock::new();
    STARTING.get_or_init(Mutex::default)
}

/// Remembers a session devpit just typed into a terminal of its own.
pub(crate) fn began(one: Starting) {
    if let Ok(mut held) = starting().lock() {
        held.retain(|was| !(was.profile == one.profile && was.name == one.name));
        held.push(one);
    }
}

/// The sessions of `profile` started here that the CLI does not list yet and
/// that are stopped on a question, as rows like the rest. One the CLI lists,
/// or that has been gone long enough, is let go.
pub(crate) fn unlisted(
    profile: &str,
    listed: &[LiveSession],
    screen: impl Fn(&str) -> Option<String>,
) -> Vec<LiveSession> {
    let Ok(mut held) = starting().lock() else {
        return Vec::new();
    };
    shown(&mut held, profile, listed, Instant::now(), screen)
}

/// [`unlisted`], over a list it is handed.
pub(crate) fn shown(
    held: &mut Vec<Starting>,
    profile: &str,
    listed: &[LiveSession],
    now: Instant,
    screen: impl Fn(&str) -> Option<String>,
) -> Vec<LiveSession> {
    held.retain(|one| {
        one.profile != profile
            || (now.duration_since(one.at) < LOOKED_FOR
                && !listed.iter().any(|live| live.name == one.name))
    });
    held.iter()
        .filter(|one| one.profile == profile)
        .filter_map(|one| {
            let waiting = crate::live_prompt::pending(&screen(&one.target())?)?;
            Some(LiveSession {
                name: one.name.clone(),
                pid: 0,
                job: None,
                status: "starting".to_owned(),
                kind: "interactive".to_owned(),
                cwd: one.cwd.clone(),
                project_id: Some(one.project_id.clone()),
                project_name: one.project_name.clone(),
                card_id: None,
                since: None,
                in_devpit: true,
                waiting: Some(waiting),
                pane: Some(LivePane {
                    project_id: one.project_id.clone(),
                    pane_id: one.pane_id.clone(),
                }),
                session_id: None,
                step: None,
                draft: None,
                remote: Default::default(),
            })
        })
        .collect()
}

/// Where the CLI lists a session `profile` starts in `project_id`: the
/// profile's own folder, and the project's account when it has one.
pub(crate) fn listed_in(profile: &str, project_id: &str) -> Vec<PathBuf> {
    let own = crate::live_sessions::config_of(profile).ok();
    let account = crate::claude_accounts::dir_of_project(project_id).map(PathBuf::from);
    own.into_iter()
        .chain(account)
        .map(|dir| dir.join("sessions"))
        .collect()
}

/// A start's answer once its session can be messaged, or saying it cannot
/// yet: the first SendMessage after a start used to be lost, and sent twice.
pub(crate) fn once_up(mut answer: Value, listed_in: &[PathBuf], within: Duration) -> Value {
    let name = answer["name"].as_str().unwrap_or_default().to_owned();
    let deadline = Instant::now() + within;
    let up = loop {
        if listed_in
            .iter()
            .any(|dir| crate::live_sessions::reachable_in(dir, &name))
        {
            break true;
        }
        if Instant::now() >= deadline {
            break false;
        }
        std::thread::sleep(LOOKED_EVERY);
    };
    answer["up"] = json!(up);
    answer["next"] = json!(if up { UP } else { NOT_UP });
    answer
}

/// The terminal of a session started here that the CLI does not list yet.
pub(crate) fn target_of(profile: &str, name: &str) -> Option<String> {
    starting()
        .lock()
        .ok()?
        .iter()
        .find(|one| one.profile == profile && one.name == name)
        .map(Starting::target)
}

#[cfg(test)]
#[path = "starting_tests.rs"]
mod tests;
