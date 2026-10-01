//! What the island knows about each agent session, folded from the hooks.
//!
//! Heard, never stored, like `card_activity`: the island draws what agents
//! are doing right now, and a row that outlived the app would still say it.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use devpit_agentcli::{Event, Happening};
use devpit_core::Store;
use devpit_rpc::{Doing, IslandChange, IslandSession, IslandStep};

use crate::card_activity::state_of_event;
use crate::card_route::{card_of_leaf, card_of_session};
use crate::listener::HookSink;

/// How many steps a session keeps: the ticker shows three, the detail six.
const STEPS: usize = 6;
/// How much of what the agent last said is kept, in characters.
const SAID: usize = 160;
/// How long a session that stopped stays on the island, in milliseconds.
const RESTING: f64 = 2.0 * 60.0 * 60.0 * 1000.0;

type Sessions = HashMap<String, IslandSession>;

pub(crate) fn registry() -> &'static Mutex<Sessions> {
    static SESSIONS: OnceLock<Mutex<Sessions>> = OnceLock::new();
    SESSIONS.get_or_init(Mutex::default)
}

/// Every session the island would draw, the most recent first.
pub(crate) fn now() -> Vec<IslandSession> {
    let Ok(sessions) = registry().lock() else {
        return Vec::new();
    };
    let mut all: Vec<IslandSession> = sessions.values().cloned().collect();
    all.sort_by(|one, other| other.at.total_cmp(&one.at));
    all
}

/// Folds one hook into its session and tells the island what changed.
pub(crate) fn heard(sink: &impl HookSink, pane: Option<&str>, happening: &Happening, at: f64) {
    let Ok(mut sessions) = registry().lock() else {
        return;
    };
    let was = sessions.remove(&happening.session_id);
    let was_state = was.as_ref().map(|known| known.state);
    // Placed once, and again only if it moved to another pane: every hook
    // asking the store which card a pane belongs to would be one query per
    // tool call for an answer that does not change.
    let fresh = was
        .as_ref()
        .is_none_or(|known| pane.is_some() && known.pane_id.as_deref() != pane);
    let placed = fresh
        .then(|| sink.store())
        .flatten()
        .map(|store| placed(&store, pane, &happening.session_id, &happening.cwd));
    let change = match fold(was, pane, happening, placed, at) {
        Some(session) => {
            sessions.insert(session.session_id.clone(), session.clone());
            IslandChange::Changed {
                session: Box::new(session),
            }
        }
        None => IslandChange::Gone {
            session_id: happening.session_id.clone(),
        },
    };
    let stale = resting(&sessions, at);
    for gone in &stale {
        sessions.remove(gone);
    }
    drop(sessions);
    // The island first: whatever telling the person costs, it is not paid by
    // the island being late.
    let told = match &change {
        IslandChange::Changed { session } => Some((session.as_ref().clone(), was_state)),
        IslandChange::Gone { .. } => None,
    };
    sink.to_island(change);
    for session_id in stale {
        sink.to_island(IslandChange::Gone { session_id });
    }
    if let Some((session, was)) = told {
        sink.tell_person(&session, was);
    }
}

/// Where a session belongs: a card, a project, or neither.
#[derive(Debug, Default, Clone, PartialEq)]
pub(crate) struct Placed {
    pub project_id: Option<String>,
    pub project: Option<String>,
    pub color: Option<String>,
    pub card_id: Option<String>,
    pub card: Option<String>,
    pub root: Option<String>,
}

fn placed(store: &Store, pane: Option<&str>, session_id: &str, cwd: &str) -> Placed {
    let card_id = match pane {
        Some(pane) => card_of_leaf(store, pane).map(|route| route.card_id),
        None => card_of_session(store, session_id).map(|(card, _)| card),
    };
    let project_id = card_id
        .as_deref()
        .and_then(|card| store.live_card_project(card).ok().flatten())
        .or_else(|| {
            let projects = store.projects().ok()?;
            project_under(
                projects
                    .iter()
                    .map(|one| (one.id.as_str(), one.root_path.as_str())),
                cwd,
            )
        });
    let project = project_id
        .as_deref()
        .and_then(|id| store.project(id).ok().flatten());
    let card = card_id
        .as_deref()
        .and_then(|card| store.card(card).ok().flatten());
    Placed {
        root: card
            .as_ref()
            .and_then(|card| card.worktree_path.clone())
            .or_else(|| project.as_ref().map(|one| one.root_path.clone())),
        card: card.map(|card| card.title),
        card_id,
        project_id,
        color: project.as_ref().and_then(|one| one.color.clone()),
        project: project.map(|one| one.name),
    }
}

/// The project whose folder holds `cwd`, the deepest when they nest, or the
/// one whose card checkout it is.
pub(crate) fn project_under<'a>(
    projects: impl Iterator<Item = (&'a str, &'a str)>,
    cwd: &str,
) -> Option<String> {
    let inside = |root: &str| {
        let root = root.trim_end_matches('/');
        !root.is_empty() && (cwd == root || cwd.starts_with(&format!("{root}/")))
    };
    let projects: Vec<(&str, &str)> = projects.collect();
    projects
        .iter()
        .filter(|(_, root)| inside(root))
        .max_by_key(|(_, root)| root.len())
        .map(|(id, _)| (*id).to_owned())
        .or_else(|| {
            // A card's worktree lives under `worktrees/<project id>/<card id>`.
            let id = cwd
                .split('/')
                .skip_while(|part| *part != "worktrees")
                .nth(1)?;
            projects
                .iter()
                .find(|(known, _)| *known == id)
                .map(|(id, _)| (*id).to_owned())
        })
}

/// One hook folded into what was known of its session; `None` once it ended.
pub(crate) fn fold(
    was: Option<IslandSession>,
    pane: Option<&str>,
    happening: &Happening,
    placed: Option<Placed>,
    at: f64,
) -> Option<IslandSession> {
    let doing = state_of_event(&happening.event);
    if doing == Some(Doing::Gone) {
        return None;
    }
    let mut session = was.unwrap_or_else(|| IslandSession {
        session_id: happening.session_id.clone(),
        pane_id: None,
        project_id: None,
        project: None,
        color: None,
        card_id: None,
        card: None,
        root: None,
        state: Doing::Open,
        steps: Vec::new(),
        said: None,
        at,
    });
    if let Some(pane) = pane {
        session.pane_id = Some(pane.to_owned());
    }
    if let Some(placed) = placed {
        session.project_id = placed.project_id;
        session.project = placed.project;
        session.color = placed.color;
        session.card_id = placed.card_id;
        session.card = placed.card;
        session.root = placed.root;
    }
    if let Some(doing) = doing {
        session.state = doing;
    }
    session.at = at;
    match &happening.event {
        // A turn begins with nothing done: working and no step is thinking.
        Event::Prompted => {
            session.steps.clear();
            session.said = None;
        }
        Event::Using {
            tool,
            target,
            touch,
        } => {
            session.steps.push(IslandStep {
                tool: tool.clone(),
                target: target.clone(),
                done: false,
                failed: false,
                touch: touch.clone(),
            });
            let over = session.steps.len().saturating_sub(STEPS);
            session.steps.drain(..over);
        }
        Event::Used { tool } | Event::UseFailed { tool } => {
            if let Some(step) = session
                .steps
                .iter_mut()
                .rev()
                .find(|step| !step.done && &step.tool == tool)
            {
                step.done = true;
                step.failed = matches!(happening.event, Event::UseFailed { .. });
            }
        }
        Event::Stopped { said } => {
            session.steps.iter_mut().for_each(|step| step.done = true);
            session.said = said.as_deref().map(cut);
        }
        Event::Failed { error } => session.said = error.as_deref().map(cut),
        Event::SessionEnded { .. } => session.steps.clear(),
        _ => {}
    }
    Some(session)
}

/// Sessions that stopped long enough ago to leave the island.
fn resting(sessions: &Sessions, at: f64) -> Vec<String> {
    sessions
        .values()
        .filter(|one| !matches!(one.state, Doing::Working | Doing::Waiting))
        .filter(|one| at - one.at > RESTING)
        .map(|one| one.session_id.clone())
        .collect()
}

fn cut(said: &str) -> String {
    let said = said.trim();
    match said.char_indices().nth(SAID) {
        Some((at, _)) => format!("{}…", &said[..at]),
        None => said.to_owned(),
    }
}

#[cfg(test)]
#[path = "island_feed_tests.rs"]
mod tests;
