//! What a card's sessions are doing, as the app hears it.
//!
//! Heard, never stored: an agent reports its state through its hooks, and a
//! row saying `working` would still say it after the app had closed and the
//! agent had finished.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use devpit_agentcli::Event;
use devpit_core::Store;
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
        Doing::Open | Doing::Failed | Doing::Gone => None,
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
                state: Some(heard.state),
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
        .filter_map(|session| session.state)
        .max_by_key(|state| rank(*state))
}

fn rank(state: Doing) -> u8 {
    match state {
        Doing::Gone => 0,
        Doing::Done => 1,
        Doing::Open => 2,
        Doing::Failed => 3,
        Doing::Working => 4,
        Doing::Waiting => 5,
    }
}

/// What a run's row says, on the card: the plan's run table.
pub(crate) fn state_of_run(state: &str) -> Doing {
    match state {
        "running" => Doing::Working,
        "failed" | "lost" => Doing::Failed,
        _ => Doing::Done,
    }
}

/// Records what a run said, and answers what the card now shows.
///
/// A card shows its latest run only: whichever run speaks, the card's other
/// runs leave it, so yesterday's failure does not outrank today's pass. Runs on
/// one card never overlap, and the next one starts after the last was heard.
pub(crate) fn hear_run(
    activities: &mut Activities,
    card_id: &str,
    reference: &str,
    seq: u64,
    state: Doing,
) -> Option<CardHappening> {
    let before = activities.heard.len();
    activities.heard.retain(|key, _| {
        key.card_id != card_id || key.kind != SessionKind::Run || key.reference == reference
    });
    let dropped = activities.heard.len() < before;
    let key = Key {
        card_id: card_id.to_owned(),
        kind: SessionKind::Run,
        reference: reference.to_owned(),
    };
    hear(activities, key, seq, state, Place::default())
        .or_else(|| dropped.then(|| activities.happening(card_id)))
}

/// `hear_run` stamped in the one order, and told to the window.
pub(crate) fn run_heard(app: &tauri::AppHandle, card_id: &str, reference: &str, state: Doing) {
    let told = registry().lock().ok().and_then(|mut activities| {
        hear_run(&mut activities, card_id, reference, next_seq(), state)
    });
    if let Some(happening) = told {
        let _ = tauri::Emitter::emit(app, "card:happening", happening);
    }
}

/// A card's conversation, as the plan's table keys it.
pub(crate) fn chat_key(card_id: &str, conversation_id: &str) -> Key {
    Key {
        card_id: card_id.to_owned(),
        kind: SessionKind::Chat,
        reference: conversation_id.to_owned(),
    }
}

/// `hear` for a session with no place on screen, stamped in the one order and
/// told to the window.
pub(crate) fn heard_now(app: &tauri::AppHandle, key: Key, state: Doing) {
    let told = registry()
        .lock()
        .ok()
        .and_then(|mut activities| hear(&mut activities, key, next_seq(), state, Place::default()));
    if let Some(happening) = told {
        let _ = tauri::Emitter::emit(app, "card:happening", happening);
    }
}

/// What a run is heard under: its agent's session, or the run itself when it
/// had none.
pub(crate) fn run_reference(store: &Store, run_id: &str) -> String {
    store
        .run_session(run_id)
        .ok()
        .flatten()
        .unwrap_or_else(|| run_id.to_owned())
}

/// Forgets one session, answering whether it was known.
pub(crate) fn forget(activities: &mut Activities, key: &Key) -> bool {
    activities.heard.remove(key).is_some()
}

/// A background session's state on the card, from its hooks and the CLI.
///
/// The CLI decides whether it still exists: one it no longer lists is gone,
/// whatever its hooks last said. While it exists, its own hooks say more than
/// the CLI's coarser word, which is only the answer when nothing was heard.
pub(crate) fn background_state(
    listed: Option<&devpit_agentcli::Status>,
    heard: Option<Doing>,
) -> Doing {
    let Some(status) = listed else {
        return Doing::Gone;
    };
    heard.unwrap_or(match status {
        devpit_agentcli::Status::Busy => Doing::Working,
        devpit_agentcli::Status::Blocked => Doing::Waiting,
        devpit_agentcli::Status::Done => Doing::Done,
        devpit_agentcli::Status::Idle | devpit_agentcli::Status::Unknown => Doing::Open,
    })
}

/// What has been heard about one card, read off the app's record.
pub(crate) fn snapshot(card_id: &str) -> CardHappening {
    registry()
        .lock()
        .map(|activities| activities.happening(card_id))
        .unwrap_or_else(|_| CardHappening {
            card_id: card_id.to_owned(),
            activity: None,
            sessions: Vec::new(),
        })
}

/// Forgets the background sessions a read found gone, so the card stops
/// carrying what their hooks last said.
pub(crate) fn prune_unlisted(card_id: &str, sessions: &[CardSession]) {
    let Ok(mut activities) = registry().lock() else {
        return;
    };
    for session in sessions
        .iter()
        .filter(|one| one.kind == SessionKind::Background && one.state == Some(Doing::Gone))
    {
        let key = Key {
            card_id: card_id.to_owned(),
            kind: SessionKind::Background,
            reference: session.reference.clone(),
        };
        forget(&mut activities, &key);
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
