//! Reminders, asked for by an agent: the orchestrator's "remind me tomorrow
//! at three", or any session's.
//!
//! devpit keeps the time and sets the reminder off, never the agent: an agent
//! keeps no timer of its own, and a reminder only tells — it starts nothing.
//! A reminder is a card with a date, so what an agent asks lands on a board,
//! where the person sees it and can move it like any other.

use std::path::Path;

use devpit_core::{ReminderRow, Store};
use devpit_rpc::{Board, Project};
use serde_json::{json, Value};

use crate::reminder_time::{instant_of, span_of};

/// The methods here, for the agent API's list.
pub(crate) const METHODS: [&str; 3] = ["remind", "reminders", "resolve_reminder"];

pub(crate) fn respond(
    app: Option<&tauri::AppHandle>,
    method: &str,
    here: &Project,
    project: &Project,
    projects: &[Project],
    params: &Value,
) -> Result<Value, String> {
    let text = |name: &str| params.get(name).and_then(Value::as_str).map(str::trim);
    let store = Store::open_default().map_err(|err| err.to_string())?;
    let answer = match method {
        "remind" => {
            let at = instant_of(text("at").ok_or("when? pass `at`, with its offset")?)?;
            if at <= now() {
                return Err("that moment has passed: a reminder is for later".to_owned());
            }
            let board = board(project)?;
            let card_id = match text("cardId").filter(|id| !id.is_empty()) {
                Some(id) => on_board(&board, id)?.to_owned(),
                None => {
                    let title = text("title")
                        .filter(|title| !title.is_empty())
                        .ok_or("a reminder needs a title, or the card it is about")?;
                    let column = board
                        .columns
                        .iter()
                        .min_by_key(|column| column.position)
                        .ok_or("this board has no columns")?;
                    crate::board::card_create_now(
                        project.id.clone(),
                        column.id.clone(),
                        title.to_owned(),
                        text("note").unwrap_or_default().to_owned(),
                    )
                    .map_err(|err| err.message)?
                    .id
                }
            };
            if !store
                .set_card_due(&card_id, Some(at), true)
                .map_err(|err| err.to_string())?
            {
                return Err("that card is archived".to_owned());
            }
            json!({
                "cardId": card_id,
                "project": project.name,
                "at": at,
                "remindersEnabled": crate::reminders::enabled(&store),
                "note": "devpit sets it off at that time, on a banner in its window and as a system notification. Say the day and time back to the person, with the zone.",
            })
        }
        "reminders" => {
            // An orchestrator asking for no project in particular sees its own
            // and its linked projects' reminders; anyone else, their project.
            let mine = project.id == here.id && here.orchestrator.is_some();
            let reach: Vec<&str> = if mine {
                let linked = crate::orchestrator_links::linked(Path::new(&here.root_path));
                projects
                    .iter()
                    .filter(|one| one.id == here.id || linked.contains(&one.id))
                    .map(|one| one.id.as_str())
                    .collect()
            } else {
                vec![project.id.as_str()]
            };
            let rows: Vec<ReminderRow> = store
                .reminders_open(None)
                .map_err(|err| err.to_string())?
                .into_iter()
                .filter(|row| reach.contains(&row.project_id.as_str()))
                .collect();
            let name = |id: &str| {
                projects
                    .iter()
                    .find(|one| one.id == id)
                    .map(|one| one.name.clone())
            };
            json!({
                "reminders": rows.iter().map(|row| json!({
                    "cardId": row.card_id,
                    "project": name(&row.project_id),
                    "title": row.title,
                    "at": row.due_at,
                    "timed": row.timed,
                    "state": if row.reminded_at.is_some() { "went off, waiting on the person" } else { "to come" },
                })).collect::<Vec<_>>(),
                "remindersEnabled": crate::reminders::enabled(&store),
            })
        }
        "resolve_reminder" => {
            let board = board(project)?;
            let card_id = on_board(&board, text("cardId").ok_or("which card? pass cardId")?)?;
            match text("action").unwrap_or_default() {
                "done" => {
                    if !store
                        .handle_reminder(card_id, now())
                        .map_err(|err| err.to_string())?
                    {
                        return Err("that card has no reminder".to_owned());
                    }
                    read_bell(&store, card_id)?;
                    json!({ "cardId": card_id, "done": true })
                }
                "cancel" => {
                    store
                        .set_card_due(card_id, None, false)
                        .map_err(|err| err.to_string())?;
                    read_bell(&store, card_id)?;
                    json!({ "cardId": card_id, "cancelled": true })
                }
                "snooze" => {
                    let until = match (text("until"), text("for")) {
                        (Some(until), _) => instant_of(until)?,
                        (None, Some(span)) => now() + span_of(span)?,
                        (None, None) => {
                            return Err("snooze until when? pass `until` or `for`".to_owned())
                        }
                    };
                    crate::reminders::snooze(&store, card_id, until as f64, now())
                        .map_err(|err| err.message)?;
                    json!({ "cardId": card_id, "at": until })
                }
                other => {
                    return Err(format!(
                        "`{other}` is not an action: done, snooze or cancel"
                    ))
                }
            }
        }
        _ => return Err(format!("devpit does not answer `{method}`")),
    };
    if method != "reminders" {
        if let Some(app) = app {
            crate::reminders::changed(app);
            let _ = tauri::Emitter::emit(app, crate::notices::RANG, ());
            let _ = tauri::Emitter::emit(app, "board:changed", &project.id);
        }
    }
    Ok(answer)
}

/// Dealt with here, it is not news in the bell either — as on the banner.
fn read_bell(store: &Store, card_id: &str) -> Result<(), String> {
    store
        .read_reminder_notices(card_id, crate::reminders::KIND, now())
        .map_err(|err| err.to_string())
}

fn board(project: &Project) -> Result<Board, String> {
    crate::board::board_get_now(project.id.clone()).map_err(|err| err.message)
}

fn on_board<'a>(board: &Board, id: &'a str) -> Result<&'a str, String> {
    board
        .cards
        .iter()
        .any(|card| card.id == id)
        .then_some(id)
        .ok_or_else(|| format!("no card `{id}` on this project's board"))
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}
