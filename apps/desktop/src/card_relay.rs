//! One card per session.
//!
//! A session that carried card after card paid for every earlier card on each
//! turn: 47% of the input billed across 16 measured cards. So a finished card
//! hands the next card of its epic to a session of its own, and a session past
//! its share of the context window leaves the handoff and is relieved by a
//! fresh one on the same card.

use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Mutex;

use devpit_core::{preference, Store};
use devpit_rpc::{Board, Card};
use serde_json::{json, Value};

/// The window the share is taken of: the 1M sessions devpit starts on Opus.
const WINDOW: u64 = 1_000_000;
const DEFAULT_PERCENT: u64 = 30;
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

/// The next card of `finished`'s epic: the first, by position, that waits in
/// a lane of waiting work and on no card not done.
pub(crate) fn next_card<'a>(board: &'a Board, finished: &str) -> Option<&'a Card> {
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
        .min_by_key(|card| card.position)
}

/// The share of the window past which a card's session hands off; `None`
/// when the person switched it off with `0` or `off`.
pub(crate) fn handoff_percent(setting: Option<&str>) -> Option<u64> {
    match setting.map(str::trim).filter(|value| !value.is_empty()) {
        None => Some(DEFAULT_PERCENT),
        Some("off" | "0") => None,
        // A value that is no share keeps the default rather than switch it off.
        Some(value) => Some(
            value
                .parse::<u64>()
                .ok()
                .filter(|n| (1..=100).contains(n))
                .unwrap_or(DEFAULT_PERCENT),
        ),
    }
}

/// Whether `context` tokens are past `percent` of the window.
pub(crate) fn past_share(context: u64, percent: u64) -> bool {
    context * 100 >= WINDOW * percent
}

/// Whether a finished card opens the next one: on unless the person said off.
pub(crate) fn relays(setting: Option<&str>) -> bool {
    setting.map(str::trim) != Some("off")
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

/// Takes the mark a session past its share left on `card_id`: true once.
fn relieved(card_id: &str) -> bool {
    let Ok(mut held) = RELIEVED.lock() else {
        return false;
    };
    let Some(relief) = held.as_mut() else {
        return false;
    };
    match relief.told.remove(card_id) {
        Some(transcript) => relief.relieved.insert(transcript),
        None => false,
    }
}

/// The account the next session runs as: the asking session's, else the
/// profile the project's reviewers run as, else Claude Code as installed.
fn profile_for(store: &Store, project_id: &str, config_dir: Option<&str>) -> String {
    crate::devflow_profile::of(store, project_id, config_dir.map(Path::new))
        .map(|profile| profile.id)
        .unwrap_or_else(|| "claude".to_owned())
}

fn first_message(card: &Card, before: Option<&str>) -> String {
    let after = before
        .map(|id| format!(" The card before it, {id}, is finished; its handoff is on that card."))
        .unwrap_or_default();
    format!(
        "Work on card {} \"{}\": read it with devpit_card and follow Devflow.{after} One card \
         per session: once its gate passes and it is finished, end the session.",
        card.id, card.title
    )
}

/// Opens `card` in a session of its own, off the caller's thread: a worktree
/// may have to be made. A session that could not open says why on the card.
fn open(app: &tauri::AppHandle, board: Board, card: Card, profile: String, before: Option<String>) {
    let app = app.clone();
    std::thread::spawn(move || {
        let prompt = first_message(&card, before.as_deref());
        if let Err(why) =
            crate::handing::hand(&app, &board, &profile, &card.id, &prompt, None, None)
        {
            if let Ok(store) = crate::projects::store() {
                let _ = store.add_comment(
                    &card.id,
                    crate::agent_api::signed(""),
                    &format!("devpit could not open a session on this card: {why}"),
                );
            }
        }
    });
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
    let next = next_card(board, card_id).filter(|_| on)?.clone();
    let profile = profile_for(&store, &board.project_id, config_dir);
    let answer = json!({
        "cardId": next.id,
        "title": next.title,
        "said": "The next card opens in a session of its own. This session's card is done: end your turn here.",
    });
    open(app, board.clone(), next, profile, Some(card_id.to_owned()));
    Some(answer)
}

/// After a handoff: a session that was told to hand off is relieved by a
/// fresh one on the same card. Answers whether one opens.
pub(crate) fn handed(
    app: Option<&tauri::AppHandle>,
    project_id: &str,
    card_id: &str,
    config_dir: Option<&str>,
) -> bool {
    let Some(app) = app else {
        return false;
    };
    if !relieved(card_id) {
        return false;
    }
    let Ok(board) = crate::board::board_get_now(project_id.to_owned()) else {
        return false;
    };
    let Some(card) = board.cards.iter().find(|card| card.id == card_id).cloned() else {
        return false;
    };
    let Ok(store) = crate::projects::store() else {
        return false;
    };
    let profile = profile_for(&store, project_id, config_dir);
    open(app, board, card, profile, None);
    true
}

#[cfg(test)]
#[path = "card_relay_tests.rs"]
mod tests;
