//! What a card's sessions are doing, as the app hears it.
//!
//! Heard, never stored: an agent reports its state through its hooks, and a
//! row saying `working` would still say it after the app had closed and the
//! agent had finished.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use devpit_agentcli::Event;
pub(crate) use devpit_rpc::Doing;
use devpit_rpc::{CardHappening, CardSession, SessionKind};

/// What a hook says about the session it fired in; `None` when it says nothing.
///
/// `/clear` ends one session and starts the next in the same agent, so it is
/// not an end.
pub(crate) fn state_of_event(event: &Event) -> Option<Doing> {
    match event {
        Event::SessionStarted => Some(Doing::Open),
        Event::Using { .. }
        | Event::Used { .. }
        | Event::SubagentStarted { .. }
        | Event::Delegated { .. }
        | Event::SubagentDone { .. } => Some(Doing::Working),
        Event::Waiting => Some(Doing::Waiting),
        Event::Stopped { .. } => Some(Doing::Done),
        Event::SessionEnded { reason } if reason.as_deref() == Some("clear") => None,
        Event::SessionEnded { .. } => Some(Doing::Gone),
    }
}

/// The word a pane's own event carries.
///
/// Panes have only ever said these three; a session beginning or ending
/// reaches the window as its own event.
pub(crate) fn pane_word(doing: Option<Doing>) -> Option<&'static str> {
    match doing? {
        Doing::Working => Some("working"),
        Doing::Waiting => Some("waiting"),
        Doing::Done => Some("done"),
        Doing::Open | Doing::Gone => None,
    }
}

static SEQ: AtomicU64 = AtomicU64::new(1);

/// The one order every source of activity is stamped in: hooks as they
/// arrive, and later runs, chats and the process table. Two counters would
/// make "newer" mean nothing between them.
pub(crate) fn next_seq() -> u64 {
    SEQ.fetch_add(1, Ordering::Relaxed)
}

/// A session of a card: its kind and the reference the plan's table gives it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub card_id: String,
    pub kind: SessionKind,
    pub reference: String,
}

/// Where a session can be reached from the card.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Place {
    pub tab_id: Option<String>,
    pub leaf_id: Option<String>,
}

struct Heard {
    seq: u64,
    state: Doing,
    place: Place,
}

/// Everything heard about cards since the app opened.
#[derive(Default)]
pub(crate) struct Activities {
    heard: HashMap<Key, Heard>,
}

impl Activities {
    /// A card's sessions as the window reads them, and what they add up to.
    pub(crate) fn happening(&self, card_id: &str) -> CardHappening {
        let mut sessions: Vec<CardSession> = self
            .heard
            .iter()
            .filter(|(key, _)| key.card_id == card_id)
            .map(|(key, heard)| CardSession {
                kind: key.kind,
                reference: key.reference.clone(),
                state: heard.state,
                tab_id: heard.place.tab_id.clone(),
                leaf_id: heard.place.leaf_id.clone(),
            })
            .collect();
        sessions.sort_by(|a, b| (a.kind, &a.reference).cmp(&(b.kind, &b.reference)));
        CardHappening {
            card_id: card_id.to_owned(),
            activity: activity(&sessions),
            sessions,
        }
    }
}

/// Records what a session said, and answers what the card now shows.
///
/// Nothing when a newer word was already heard — posts are served on threads
/// that finish in any order — or when the state did not change, which keeps a
/// session saying `working` before every tool from repainting the board.
pub(crate) fn hear(
    activities: &mut Activities,
    key: Key,
    seq: u64,
    state: Doing,
    place: Place,
) -> Option<CardHappening> {
    if let Some(known) = activities.heard.get_mut(&key) {
        if known.seq > seq {
            return None;
        }
        if known.state == state {
            known.seq = seq;
            return None;
        }
    }
    let card_id = key.card_id.clone();
    activities.heard.insert(key, Heard { seq, state, place });
    Some(activities.happening(&card_id))
}

/// Takes a closed pane off its card, and answers what the card shows now.
///
/// A closed window posts nothing more, so without this the pane would stay on
/// the card as whatever it last said — `working`, forever.
pub(crate) fn forget_leaf(activities: &mut Activities, leaf: &str) -> Option<CardHappening> {
    let key = activities
        .heard
        .keys()
        .find(|key| key.kind == SessionKind::Pane && key.reference == leaf)
        .cloned()?;
    activities.heard.remove(&key);
    Some(activities.happening(&key.card_id))
}

/// `forget_leaf` for the panes the window just closed, told to the window.
pub(crate) fn panes_closed(app: &tauri::AppHandle, leaves: &[String]) {
    let Ok(mut activities) = registry().lock() else {
        return;
    };
    for leaf in leaves {
        if let Some(happening) = forget_leaf(&mut activities, leaf) {
            let _ = tauri::Emitter::emit(app, "card:happening", happening);
        }
    }
}

/// What a card's sessions add up to on its tile: the one most worth looking at.
pub(crate) fn activity(sessions: &[CardSession]) -> Option<Doing> {
    sessions
        .iter()
        .map(|session| session.state)
        .max_by_key(|state| rank(*state))
}

fn rank(state: Doing) -> u8 {
    match state {
        Doing::Gone => 0,
        Doing::Done => 1,
        Doing::Open => 2,
        Doing::Working => 3,
        Doing::Waiting => 4,
    }
}

/// The app's own record. Only the listener's `AppHandle` sink reaches it; the
/// rules above take an `Activities` so no test shares one.
pub(crate) fn registry() -> &'static Mutex<Activities> {
    static ACTIVITIES: OnceLock<Mutex<Activities>> = OnceLock::new();
    ACTIVITIES.get_or_init(Mutex::default)
}

#[cfg(test)]
#[path = "card_activity_tests.rs"]
mod tests;
