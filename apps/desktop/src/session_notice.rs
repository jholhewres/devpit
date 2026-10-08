//! What an orchestrator's sessions did since its last turn, said at the top
//! of its next one.
//!
//! A session that finishes does not wake the orchestrator: a turn it did not
//! ask for spends the person's money and talks over them. devpit remembers
//! how the sessions stood when the orchestrator last spoke, and the next turn
//! the person starts begins with the difference — "since your last message:
//! api-worker finished its turn" — so the orchestrator is never the last to
//! know and never speaks unasked.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use devpit_rpc::LiveSession;

/// How the sessions stood at a turn.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Seen {
    /// Milliseconds since the epoch.
    pub at: f64,
    pub names: BTreeSet<String>,
    pub busy: BTreeSet<String>,
    pub waiting: BTreeSet<String>,
}

impl Seen {
    pub(crate) fn of(sessions: &[LiveSession], at: f64) -> Seen {
        let named = |keep: &dyn Fn(&LiveSession) -> bool| {
            sessions
                .iter()
                .filter(|one| keep(one))
                .map(|one| one.name.clone())
                .collect()
        };
        Seen {
            at,
            names: named(&|_| true),
            busy: named(&|one| one.status == "busy"),
            waiting: named(&|one| one.waiting.is_some()),
        }
    }
}

fn seen() -> &'static Mutex<HashMap<String, Seen>> {
    static SEEN: OnceLock<Mutex<HashMap<String, Seen>>> = OnceLock::new();
    SEEN.get_or_init(Mutex::default)
}

/// What changed between `before` and `now`, in a line, or nothing.
pub(crate) fn notice(before: Option<&Seen>, now: &[LiveSession]) -> Option<String> {
    // The first turn has nothing to compare with.
    let before = before?;
    let mut said = Vec::new();
    for one in now {
        let new = !before.names.contains(&one.name);
        if one.waiting.is_some() && !before.waiting.contains(&one.name) {
            said.push(format!("{} is waiting on a question", one.name));
        } else if one.status == "idle"
            && (before.busy.contains(&one.name)
                || (new && one.since.is_some_and(|since| since > before.at)))
        {
            said.push(format!("{} finished its turn", one.name));
        }
    }
    let here: BTreeSet<&String> = now.iter().map(|one| &one.name).collect();
    for gone in before.names.iter().filter(|name| !here.contains(name)) {
        said.push(format!("{gone} ended"));
    }
    (!said.is_empty()).then(|| {
        format!(
            "[devpit, not the person] Since your last message: {}.",
            said.join("; ")
        )
    })
}

/// `prompt`, with what changed since the last turn above it when the chat is
/// an orchestrator's; the prompt as it is otherwise.
pub(crate) fn with_notice(project_id: &str, profile_id: &str, prompt: &str, now_ms: f64) -> String {
    let Some(here) = crate::projects::project_list_now().ok().and_then(|listed| {
        listed
            .projects
            .into_iter()
            .find(|one| one.id == project_id && one.orchestrator.is_some())
    }) else {
        return prompt.to_owned();
    };
    let Ok(live) = crate::live_sessions::orchestrator_sessions_now(profile_id) else {
        return prompt.to_owned();
    };
    let sessions = crate::orchestrator_links::within(
        &here.id,
        &crate::orchestrator_links::linked(Path::new(&here.root_path)),
        live.sessions,
    );
    // The person speaks: the round the orchestrator reads after a compaction.
    crate::round_state::keep(Path::new(&here.root_path), &here.id, profile_id, &sessions);
    let Ok(mut held) = seen().lock() else {
        return prompt.to_owned();
    };
    let said = notice(held.get(project_id), &sessions);
    held.insert(project_id.to_owned(), Seen::of(&sessions, now_ms));
    match said {
        Some(said) => format!("{said}\n\n{prompt}"),
        None => prompt.to_owned(),
    }
}

#[cfg(test)]
#[path = "session_notice_tests.rs"]
mod tests;
