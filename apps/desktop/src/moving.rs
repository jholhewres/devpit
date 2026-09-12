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
/// nothing, and an irreversible one runs nothing until someone says so.
fn what_runs(step: Option<&Step>, confirmed: bool) -> Option<&Step> {
    match step {
        // No step on this column: moving the card is all that happened.
        None => None,
        // Irreversible and unconfirmed: the move stands, the work does not.
        Some(step) if step.irreversible && !confirmed => None,
        Some(step) => Some(step),
    }
}

/// `card.move` — and the only place a step is ever started.
///
/// `confirmed` is how an irreversible step stays out of a drag: a deploy is
/// not fired by dropping a card on a lane, it is fired by someone saying so.
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

    store.move_card(&card_id, &column_id, position as i64)?;

    let steps = steps_of(&store, &project_id)?;
    let landed = store
        .columns(&project_id)?
        .into_iter()
        .find(|column| column.id == column_id);

    let step = landed
        .and_then(|column| column.step_id)
        .and_then(|id| steps.iter().find(|s| s.id == id).cloned());

    let started = match what_runs(step.as_ref(), confirmed) {
        None => None,
        Some(step) => {
            // The one kind of run worth a line in the bell at the moment it
            // *starts*. Everything else is told when it ends; a deploy that
            // was just set off is a thing to be able to see was set off, by
            // whom and on what, without waiting for it to come back.
            if step.irreversible {
                let title = store
                    .card(&card_id)
                    .ok()
                    .flatten()
                    .map(|row| row.title)
                    .unwrap_or_else(|| "a card".to_owned());
                crate::notices::ring(
                    &app,
                    Some(&project_id),
                    crate::notices::kind::IRREVERSIBLE,
                    &format!("{} started on “{title}”", step.name),
                    Some("This step was marked as having no undo."),
                    Some(&card_id),
                );
            }
            Some(crate::runs::start(
                app,
                Arc::clone(&in_flight),
                &store,
                &card_id,
                step,
                came_from.as_deref(),
            )?)
        }
    };

    Ok(CardChanged {
        card: card_of(&store, &card_id, &steps)?,
        started,
    })
}

#[cfg(test)]
#[path = "moving_tests.rs"]
mod tests;
