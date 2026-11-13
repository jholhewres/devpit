//! One card per session, when the person turns it on.
//!
//! A session that carried card after card paid for every earlier card on each
//! turn: 47% of the input billed across 16 measured cards. So, when switched
//! on in Settings → Sessions, a finished card hands the next card of its epic
//! to a session of its own, and a session past its share of the context window
//! leaves the handoff and is relieved by a fresh one in the same folder. Both
//! are off unless set: devpit starts no session on its own.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use devpit_core::{preference, Store};
use devpit_rpc::{Board, Card};
use serde_json::{json, Value};

/// The window the share is taken of: the 1M sessions devpit starts on Opus.
const WINDOW: u64 = 1_000_000;
/// How much of a transcript's end is read for its last usage.
const TAIL: u64 = 256 * 1024;

/// The tools a session past its share still has: enough to read its card and
/// leave the handoff.
const STILL_ALLOWED: [&str; 4] = [
    "devpit_handoff",
    "devpit_comment",
    "devpit_card",
    "devpit_context",
];

/// Cards whose session was told to hand off, by the transcript told, and the
/// transcripts already relieved: one session hands off once.
#[derive(Default)]
struct Relief {
    told: HashMap<String, String>,
    relieved: HashSet<String>,
}

static RELIEVED: Mutex<Option<Relief>> = Mutex::new(None);

/// Cards the relay opened in this run.
static OPENED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Whether the relay may open `card_id`: once, since its turn can come both
/// at the finish of the card before it and when that card enters done.
pub(crate) fn first_opening(card_id: &str) -> bool {
    let Ok(mut opened) = OPENED.lock() else {
        return false;
    };
    if opened.iter().any(|id| id == card_id) {
        return false;
    }
    opened.push(card_id.to_owned());
    true
}

/// The card to open once `card_id` moved from the lane `from` to where
/// `board` has it: the next card of its epic that waited on it, when it
/// entered done. With `unblock_on: done` that is when such a card is released;
/// a card free all along was the finish's to open.
pub(crate) fn next_on_entering<'a>(
    board: &'a Board,
    card_id: &str,
    from: Option<&str>,
) -> Option<&'a Card> {
    let done = |lane: &str| {
        board
            .columns
            .iter()
            .any(|column| column.id == lane && column.role.as_deref() == Some("done"))
    };
    let card = board.cards.iter().find(|card| card.id == card_id)?;
    if !done(&card.column_id) || from.is_some_and(done) {
        return None;
    }
    next_where(board, card_id, |next| {
        next.blocked_by.iter().any(|by| by.id == card_id)
    })
}

/// The next card of `finished`'s epic: the first, by position, that waits in
/// a lane of waiting work and on no card not done.
pub(crate) fn next_card<'a>(board: &'a Board, finished: &str) -> Option<&'a Card> {
    next_where(board, finished, |_| true)
}

/// [`next_card`], among the cards `wanted` keeps.
fn next_where<'a>(
    board: &'a Board,
    finished: &str,
    wanted: impl Fn(&Card) -> bool,
) -> Option<&'a Card> {
    let parent = board
        .cards
        .iter()
        .find(|card| card.id == finished)?
        .parent_id
        .as_deref()?;
    let waiting: Vec<&str> = board
        .columns
        .iter()
        .filter(|column| column.role.as_deref() == Some("backlog") && column.step.is_none())
        .map(|column| column.id.as_str())
        .collect();
    board
        .cards
        .iter()
        .filter(|card| card.id != finished && card.parent_id.as_deref() == Some(parent))
        .filter(|card| waiting.contains(&card.column_id.as_str()))
        .filter(|card| !card.blocked_by.iter().any(|by| by.holds()))
        .filter(|card| wanted(card))
        .min_by_key(|card| card.position)
}

/// The share of the window past which a card's session hands off; `None`,
/// the default, unless the person set a share from 1 to 100.
pub(crate) fn handoff_percent(setting: Option<&str>) -> Option<u64> {
    setting?
        .trim()
        .parse::<u64>()
        .ok()
        .filter(|n| (1..=100).contains(n))
}

/// Whether `context` tokens are past `percent` of the window.
pub(crate) fn past_share(context: u64, percent: u64) -> bool {
    context * 100 >= WINDOW * percent
}

/// Whether a finished card opens the next one: only when the person said on.
pub(crate) fn relays(setting: Option<&str>) -> bool {
    setting.map(str::trim) == Some("on")
}

/// The context the session last read: the newest main-thread assistant
/// message's input, cache reads and cache writes.
pub(crate) fn context_in(tail: &str) -> Option<u64> {
    tail.lines().rev().find_map(|line| {
        let row: Value = serde_json::from_str(line).ok()?;
        if row.get("type")?.as_str()? != "assistant" || row["isSidechain"] == json!(true) {
            return None;
        }
        let usage = row.get("message")?.get("usage")?;
        let count = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
        Some(
            count("input_tokens")
                + count("cache_read_input_tokens")
                + count("cache_creation_input_tokens"),
        )
    })
}

fn tail_of(path: &Path) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(len.saturating_sub(TAIL))).ok()?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).ok()?;
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// Whether `tool` is one a session past its share may still call.
pub(crate) fn still_allowed(tool: &str) -> bool {
    STILL_ALLOWED
        .iter()
        .any(|name| tool == *name || tool.ends_with(&format!("__{name}")))
}

/// What a session past its share is told instead of running a tool.
pub(crate) fn hand_off_reason(context: u64, percent: u64) -> String {
    format!(
        "This session holds {}k tokens, past {percent}% of its context window. One card per \
         session: leave the handoff now with devpit_handoff (what is done, what is left, how to \
         test it, the risks), then end your turn. devpit opens a fresh session on this card \
         from that handoff.",
        context / 1000
    )
}

/// The PreToolUse reply for a card's session past its share, or `None` to
/// leave the tool to the CLI.
pub(crate) fn over_share(store: &Store, body: &str, pane: &str) -> Option<String> {
    let hook: Value = serde_json::from_str(body).ok()?;
    if hook.get("hook_event_name")?.as_str()? != "PreToolUse"
        || still_allowed(hook.get("tool_name")?.as_str()?)
    {
        return None;
    }
    let percent = handoff_percent(
        store
            .preference(preference::SESSION_HANDOFF_PERCENT)
            .ok()
            .flatten()
            .as_deref(),
    )?;
    // Only a card's session hands off: an orchestrator's has no card to leave.
    let route = crate::card_route::card_of_leaf(store, pane)?;
    let transcript = hook.get("transcript_path")?.as_str()?;
    let context = context_in(&tail_of(Path::new(transcript))?)?;
    if !past_share(context, percent) {
        return None;
    }
    mark(&route.card_id, transcript);
    let reason = serde_json::to_string(&hand_off_reason(context, percent)).ok()?;
    Some(format!(
        r#"{{"hookSpecificOutput":{{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":{reason}}}}}"#
    ))
}

fn mark(card_id: &str, transcript: &str) {
    if let Ok(mut held) = RELIEVED.lock() {
        let relief = held.get_or_insert_with(Relief::default);
        if !relief.relieved.contains(transcript) {
            relief
                .told
                .insert(card_id.to_owned(), transcript.to_owned());
        }
    }
}

/// Takes the mark a session past its share left on `card_id`, once: the
/// transcript of the session to relieve.
fn relieved(card_id: &str) -> Option<String> {
    let mut held = RELIEVED.lock().ok()?;
    let relief = held.as_mut()?;
    let transcript = relief.told.remove(card_id)?;
    relief
        .relieved
        .insert(transcript.clone())
        .then_some(transcript)
}

/// The account the next session runs as: the asking session's, else the
/// profile the project's reviewers run as, else Claude Code as installed.
fn profile_for(store: &Store, project_id: &str, config_dir: Option<&str>) -> String {
    crate::devflow_profile::of(store, project_id, config_dir.map(Path::new))
        .map(|profile| profile.id)
        .unwrap_or_else(|| "claude".to_owned())
}

/// `one_per_session` only while the next card opens on its own: told so
/// otherwise, a session refused the next card the orchestrator gave it.
fn first_message(card: &Card, before: Option<&str>, one_per_session: bool) -> String {
    let after = before
        .map(|id| format!(" The card before it, {id}, is finished; its handoff is on that card."))
        .unwrap_or_default();
    let alone = if one_per_session {
        " One card per session: once its gate passes and it is finished, end the session."
    } else {
        ""
    };
    format!(
        "Work on card {} \"{}\": read it with devpit_card and follow Devflow.{after}{alone}",
        card.id, card.title
    )
}

/// The session a relief replaces, by the id its transcript is named after.
fn replaced_id(transcript: &str) -> Option<&str> {
    Path::new(transcript)
        .file_stem()?
        .to_str()
        .filter(|id| crate::adopting::plain(id))
}

/// How a session is opened on a card.
struct Opening {
    profile: String,
    before: Option<String>,
    one_per_session: bool,
    /// The folder of the session it relieves: the relief works where it did.
    folder: Option<std::path::PathBuf>,
    /// The session it relieves, by id: stopped once the new one is up.
    replaces: Option<String>,
}

/// Opens `card` in a session of its own, off the caller's thread: a worktree
/// may have to be made. A session that could not open says why on the card.
fn open(app: &tauri::AppHandle, board: Board, card: Card, how: Opening) {
    let app = app.clone();
    std::thread::spawn(move || {
        let prompt = first_message(&card, how.before.as_deref(), how.one_per_session);
        let started = crate::handing::hand(
            &app,
            &board,
            &how.profile,
            &card.id,
            &prompt,
            Default::default(),
            how.folder.as_deref(),
        );
        let started = match started {
            Ok(started) => started,
            Err(why) => {
                if let Ok(store) = crate::projects::store() {
                    let _ = store.add_comment(
                        &card.id,
                        crate::agent_api::signed(""),
                        &format!("devpit could not open a session on this card: {why}"),
                    );
                }
                return;
            }
        };
        if let Some(old) = how.replaces {
            let up = crate::starting::once_up(
                started,
                &crate::starting::listed_in(&how.profile, &board.project_id),
                crate::starting::UP_WITHIN,
            );
            if up["up"] == json!(true) {
                stop_replaced(&app, &how.profile, &old);
            }
        }
    });
}

/// Stops the session a relief took over from, by its id.
fn stop_replaced(app: &tauri::AppHandle, profile: &str, session_id: &str) {
    let Ok(live) = crate::live_sessions::orchestrator_sessions_now(profile) else {
        return;
    };
    if let Some(old) = live
        .sessions
        .iter()
        .find(|one| one.session_id.as_deref() == Some(session_id))
    {
        let _ = crate::stopping::stop(app, profile, &old.name, Some(old.pid), "devpit", true);
    }
}

/// After `finish_card`: the next card of the epic opens in a session of its
/// own. Answers what opened, for the finishing session to stop on.
pub(crate) fn finished(
    app: Option<&tauri::AppHandle>,
    board: &Board,
    card_id: &str,
    config_dir: Option<&str>,
) -> Option<Value> {
    let app = app?;
    let store = crate::projects::store().ok()?;
    let on = relays(
        store
            .preference(preference::SESSION_NEXT_CARD)
            .ok()
            .flatten()
            .as_deref(),
    );
    let next = next_card(board, card_id)
        .filter(|next| on && first_opening(&next.id))?
        .clone();
    let profile = profile_for(&store, &board.project_id, config_dir);
    let answer = json!({
        "cardId": next.id,
        "title": next.title,
        "said": "The next card opens in a session of its own. This session's card is done: end your turn here.",
    });
    let how = Opening {
        profile,
        before: Some(card_id.to_owned()),
        one_per_session: true,
        folder: None,
        replaces: None,
    };
    open(app, board.clone(), next, how);
    Some(answer)
}

/// After a move: a card that entered done opens the next card of its epic in
/// a session of its own, when the person turned that on.
pub(crate) fn moved(app: &tauri::AppHandle, project_id: &str, card_id: &str, from: Option<&str>) {
    let Ok(store) = crate::projects::store() else {
        return;
    };
    let on = relays(
        store
            .preference(preference::SESSION_NEXT_CARD)
            .ok()
            .flatten()
            .as_deref(),
    );
    if !on {
        return;
    }
    let (app, project_id, card_id) = (app.clone(), project_id.to_owned(), card_id.to_owned());
    let from = from.map(str::to_owned);
    std::thread::spawn(move || {
        let Ok(board) = crate::board::board_get_now(project_id.clone()) else {
            return;
        };
        let Some(next) = next_on_entering(&board, &card_id, from.as_deref())
            .filter(|next| first_opening(&next.id))
            .cloned()
        else {
            return;
        };
        let Ok(store) = crate::projects::store() else {
            return;
        };
        let how = Opening {
            profile: profile_for(&store, &project_id, None),
            before: Some(card_id),
            one_per_session: true,
            folder: None,
            replaces: None,
        };
        open(&app, board, next, how);
    });
}

/// After a handoff: a session that was told to hand off is relieved by a
/// fresh one on the same card, in `folder`, where it ran; once that one is
/// up, it is stopped. Answers whether one opens.
pub(crate) fn handed(
    app: Option<&tauri::AppHandle>,
    project_id: &str,
    card_id: &str,
    config_dir: Option<&str>,
    folder: &str,
) -> bool {
    let Some(app) = app else {
        return false;
    };
    let Some(transcript) = relieved(card_id) else {
        return false;
    };
    let Ok(board) = crate::board::board_get_now(project_id.to_owned()) else {
        return false;
    };
    let Some(card) = board.cards.iter().find(|card| card.id == card_id).cloned() else {
        return false;
    };
    let Ok(store) = crate::projects::store() else {
        return false;
    };
    let how = Opening {
        profile: profile_for(&store, project_id, config_dir),
        before: None,
        one_per_session: relays(
            store
                .preference(preference::SESSION_NEXT_CARD)
                .ok()
                .flatten()
                .as_deref(),
        ),
        folder: (!folder.is_empty()).then(|| std::path::PathBuf::from(folder)),
        replaces: replaced_id(&transcript).map(str::to_owned),
    };
    open(app, board, card, how);
    true
}

/// The two switches as the settings screen shows them.
pub(crate) fn relay_of(next_card: Option<&str>, handoff: Option<&str>) -> devpit_rpc::SessionRelay {
    devpit_rpc::SessionRelay {
        next_card: relays(next_card),
        handoff_at_percent: handoff_percent(handoff).and_then(|n| u32::try_from(n).ok()),
    }
}

fn relay_read(store: &Store) -> Result<devpit_rpc::SessionRelay, devpit_rpc::RpcError> {
    Ok(relay_of(
        store.preference(preference::SESSION_NEXT_CARD)?.as_deref(),
        store
            .preference(preference::SESSION_HANDOFF_PERCENT)?
            .as_deref(),
    ))
}

/// `sessions.relay_read` — whether devpit starts sessions on its own.
#[tauri::command]
#[specta::specta]
pub async fn session_relay_read() -> Result<devpit_rpc::SessionRelay, devpit_rpc::RpcError> {
    crate::off_main::blocking(|| relay_read(&crate::projects::store()?)).await
}

/// `sessions.relay_write` — both switches at once, answered as stored.
#[tauri::command]
#[specta::specta]
pub async fn session_relay_write(
    relay: devpit_rpc::SessionRelay,
) -> Result<devpit_rpc::SessionRelay, devpit_rpc::RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let next = if relay.next_card { "on" } else { "off" };
        store.set_preference(preference::SESSION_NEXT_CARD, next)?;
        let share = relay
            .handoff_at_percent
            .filter(|n| (1..=100).contains(n))
            .map_or_else(|| "off".to_owned(), |n| n.to_string());
        store.set_preference(preference::SESSION_HANDOFF_PERCENT, &share)?;
        crate::sync::nudge();
        relay_read(&store)
    })
    .await
}

#[cfg(test)]
#[path = "card_relay_tests.rs"]
mod tests;
