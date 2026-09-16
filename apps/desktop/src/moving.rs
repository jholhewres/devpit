//! Moving a card, and the one rule the whole product turns on.
//!
//! Apart from `board.rs` because that file assembles the board and this one
//! changes it: a card landing on a column is the only gesture in the product
//! that starts work, and the rule deciding whether it does is worth reading
//! on its own.

use std::sync::Arc;

use devpit_rpc::{CardChanged, ErrorCode, RpcError, Step};
use tauri::{AppHandle, State};

use crate::board::{card_of, steps_of, store};

/// What a card landing on a column sets off, if anything.
///
/// The rule the whole product turns on, kept as a function of its own so it
/// can be read and tested without a window: a column with no step runs
/// nothing, and an irreversible one never runs from a move. It is started by
/// the card's play button, which asks about that step and nothing else —
/// "move it anyway" answers a question about the work already going, and
/// taking it as consent to a deploy fired one nobody was asked about.
fn what_runs(step: Option<&Step>) -> Option<&Step> {
    match step {
        // No step on this column: moving the card is all that happened.
        None => None,
        // Irreversible: the move stands, the work waits for the play button.
        Some(step) if step.irreversible => None,
        Some(step) => Some(step),
    }
}

/// Why a move is refused before the card is written, if it is: only when the
/// lane would start a step that an update in progress refuses.
fn refused_before_moving(
    step: Option<&Step>,
    update: Option<&devpit_rpc::UpdateStatus>,
) -> Option<&'static str> {
    what_runs(step)?;
    crate::update::starting_refused(update?)
}

/// `card.move` — and the only place a step is ever started.
///
/// `confirmed` answers one question: move the card although a run is still
/// going on it. It is never consent to an irreversible step.
#[tauri::command]
#[specta::specta]
pub fn card_move(
    in_flight: State<Arc<crate::in_flight::InFlight>>,
    app: AppHandle,
    project_id: String,
    card_id: String,
    column_id: String,
    position: i32,
    confirmed: bool,
) -> Result<CardChanged, RpcError> {
    let store = store()?;

    // Read before the move: a step that sends the card back needs somewhere to
    // send it, and after the write the previous column is gone.
    let came_from = store.card(&card_id)?.map(|card| card.column_id);

    // A run still going is work in flight. Moving the card out from under it
    // is allowed, but only on purpose.
    let still_running = store
        .runs(&card_id)?
        .iter()
        .any(|run| run.state == "running");
    if still_running && !confirmed {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "a run is still going on this card — move it anyway?",
        ));
    }

    let steps = steps_of(&store, &project_id)?;
    let landed = store
        .columns(&project_id)?
        .into_iter()
        .find(|column| column.id == column_id);

    let step = landed
        .and_then(|column| column.step_id)
        .and_then(|id| steps.iter().find(|s| s.id == id).cloned());

    // Refused before anything is written: a move whose step an update would
    // refuse used to land in the store while the board put the card back.
    let update =
        tauri::Manager::try_state::<crate::update::Updating>(&app).map(|updating| updating.state());
    // Busy, not Conflict: a conflict is the question "move it anyway?", and
    // no answer to it gets past an update that is going in.
    if let Some(why) = refused_before_moving(step.as_ref(), update.as_ref()) {
        return Err(RpcError::new(ErrorCode::Busy, why.to_owned()));
    }

    store.move_card(&card_id, &column_id, position as i64)?;

    let started = match what_runs(step.as_ref()) {
        None => None,
        // Never an irreversible step: `what_runs` sends those to the play
        // button, which is where the bell for one rings.
        Some(step) => Some(crate::runs::start(
            app,
            Arc::clone(&in_flight),
            &store,
            &card_id,
            step,
            came_from.as_deref(),
        )?),
    };

    Ok(CardChanged {
        card: card_of(&store, &card_id, &steps)?,
        started,
    })
}

#[cfg(test)]
#[path = "moving_tests.rs"]
mod tests;
