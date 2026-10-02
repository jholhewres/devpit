//! The card follows the work, by what its columns are for.
//!
//! An agent was told to move its card "to the column where work in progress
//! sits" and had to find that column by name, on a board that names its
//! columns its own way — so it forgot, or chose wrong, and the board stopped
//! saying where anything was. Now a column says what it is for
//! (`column_roles`), an agent says "I started" or "I finished" without naming
//! a column, and devpit moves a waiting card to work in progress itself the
//! moment its session takes a prompt. Never into a column that runs a step:
//! entering one starts work, and that stays the person's decision.

use devpit_rpc::{Board, Column};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

/// The column holding `role` on this board: the first one, by position,
/// that runs no step.
pub(crate) fn column_with<'a>(board: &'a Board, role: &str) -> Option<&'a Column> {
    board
        .columns
        .iter()
        .filter(|column| column.role.as_deref() == Some(role) && column.step.is_none())
        .min_by_key(|column| column.position)
}

/// Where a card goes when work starts on it, if it goes anywhere: only out of
/// a column of waiting work, into the one for work in progress.
pub(crate) fn when_started<'a>(board: &'a Board, card_id: &str) -> Option<&'a Column> {
    let card = board.cards.iter().find(|card| card.id == card_id)?;
    let here = board
        .columns
        .iter()
        .find(|column| column.id == card.column_id)?;
    (here.role.as_deref() == Some("backlog"))
        .then(|| column_with(board, "doing"))
        .flatten()
}

/// `start_card`: the card goes to work in progress, wherever it waited.
pub(crate) fn start(
    app: Option<&AppHandle>,
    board: &Board,
    card_id: &str,
) -> Result<Value, String> {
    let to = column_with(board, "doing").ok_or(
        "this board has no column for work in progress that runs no step: say which with devpit_move_card, or leave the card where it is",
    )?;
    crate::agent_api::moved(app, board, card_id, &to.id)
}

/// `finish_card`: what was done, said on the card, and the card to the
/// column where finished work waits to be checked.
pub(crate) fn finish(
    app: Option<&AppHandle>,
    board: &Board,
    card_id: &str,
    summary: &str,
    author: &str,
) -> Result<Value, String> {
    let to = column_with(board, "check").ok_or(
        "this board has no column for work to check that runs no step: say which with devpit_move_card, or leave the card where it is",
    )?;
    let summary = summary.trim();
    if !summary.is_empty() {
        crate::projects::store()
            .map_err(|err| err.message)?
            .add_comment(card_id, crate::agent_api::signed(author), summary)
            .map_err(|err| err.to_string())?;
    }
    crate::agent_api::moved(app, board, card_id, &to.id)
}

/// A session of the card took a prompt: a waiting card moves to work in
/// progress. On its own thread — a hook is waiting on the reply.
pub(crate) fn prompted(app: &AppHandle, card_id: &str) {
    let app = app.clone();
    let card_id = card_id.to_owned();
    std::thread::spawn(move || {
        let Some(project) = crate::projects::store()
            .ok()
            .and_then(|store| store.live_card_project(&card_id).ok().flatten())
        else {
            return;
        };
        let Ok(board) = crate::board::board_get_now(project.clone()) else {
            return;
        };
        if let Some(to) = when_started(&board, &card_id) {
            if crate::agent_api::moved(Some(&app), &board, &card_id, &to.id).is_ok() {
                let _ = app.emit("board:changed", &project);
            }
        }
    });
}

/// What `devpit_context` says about the columns: each one's role, so an agent
/// knows where its card goes without guessing from names.
pub(crate) fn roles(board: &Board) -> Value {
    json!({
        "doing": column_with(board, "doing").map(|column| &column.id),
        "check": column_with(board, "check").map(|column| &column.id),
    })
}

#[cfg(test)]
#[path = "card_follows_tests.rs"]
mod tests;
