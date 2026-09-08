//! Board commands: the columns, the cards, and what a move sets off.
//!
//! Moving a card into a column that has a step is what starts work. Moving it
//! into one that does not is just a move. That is the whole rule, and there is
//! no other way to start a step: no schedule, no retry, no loop that keeps
//! itself going.

use quockpit_core::Store;
use quockpit_rpc::{
    Board, Card, CardChanged, Column, ErrorCode, RpcError, Run, RunState, Step, StepKind,
};
use tauri::AppHandle;

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

fn kind_of(raw: &str) -> StepKind {
    match raw {
        "session" => StepKind::Session,
        "command" => StepKind::Command,
        _ => StepKind::Agent,
    }
}

fn state_of(raw: &str) -> RunState {
    match raw {
        "ok" => RunState::Ok,
        "failed" => RunState::Failed,
        "cancelled" => RunState::Cancelled,
        _ => RunState::Running,
    }
}

fn steps_of(store: &Store, project_id: &str) -> Result<Vec<Step>, RpcError> {
    Ok(store
        .steps(project_id)?
        .into_iter()
        .map(|row| Step {
            id: row.id,
            kind: kind_of(&row.kind),
            name: row.name,
            config: row.config,
            irreversible: row.irreversible,
        })
        .collect())
}

fn card_of(store: &Store, id: &str, steps: &[Step]) -> Result<Card, RpcError> {
    let row = store
        .card(id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;
    Ok(Card {
        id: row.id.clone(),
        column_id: row.column_id,
        title: row.title,
        body: row.body,
        position: row.position as i32,
        worktree_path: row.worktree_path,
        cost_usd: store.card_cost(&row.id)?,
        runs: runs_of(store, &row.id, steps)?,
    })
}

fn runs_of(store: &Store, card_id: &str, steps: &[Step]) -> Result<Vec<Run>, RpcError> {
    Ok(store
        .runs(card_id)?
        .into_iter()
        .map(|row| Run {
            step_name: steps
                .iter()
                .find(|s| s.id == row.step_id)
                .map(|s| s.name.clone())
                // A step deleted after its run leaves the run standing. The
                // history of what happened does not disappear with the recipe.
                .unwrap_or_else(|| "a deleted step".to_owned()),
            id: row.id,
            step_id: row.step_id,
            state: state_of(&row.state),
            output: row.output,
            exit_code: row.exit_code.map(|c| c as i32),
            cost_usd: row.cost_usd,
            duration_ms: row.duration_ms.map(|ms| ms as f64),
            started_at: row.started_at as f64,
        })
        .collect())
}

/// `board.get` — the columns, the cards and the steps of a project.
#[tauri::command]
#[specta::specta]
pub fn board_get(project_id: String) -> Result<Board, RpcError> {
    let store = store()?;
    store.ensure_board(&project_id)?;
    let steps = steps_of(&store, &project_id)?;

    let columns = store
        .columns(&project_id)?
        .into_iter()
        .map(|row| Column {
            id: row.id,
            name: row.name,
            position: row.position as i32,
            step: row
                .step_id
                .and_then(|id| steps.iter().find(|s| s.id == id).cloned()),
        })
        .collect();

    let mut cards = Vec::new();
    for row in store.cards(&project_id)? {
        cards.push(card_of(&store, &row.id, &steps)?);
    }

    Ok(Board {
        project_id,
        columns,
        cards,
        steps,
    })
}

#[tauri::command]
#[specta::specta]
pub fn card_create(
    project_id: String,
    column_id: String,
    title: String,
    body: String,
) -> Result<Card, RpcError> {
    let store = store()?;
    let id = store.create_card(&project_id, &column_id, &title, &body)?;
    let steps = steps_of(&store, &project_id)?;
    card_of(&store, &id, &steps)
}

#[tauri::command]
#[specta::specta]
pub fn card_update(
    project_id: String,
    card_id: String,
    title: String,
    body: String,
) -> Result<Card, RpcError> {
    let store = store()?;
    store.update_card(&card_id, &title, &body)?;
    let steps = steps_of(&store, &project_id)?;
    card_of(&store, &card_id, &steps)
}

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
    let in_flight = store
        .runs(&card_id)?
        .iter()
        .any(|run| run.state == "running");
    if in_flight && !confirmed {
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
        Some(step) => Some(crate::runs::start(
            app,
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
#[path = "board_tests.rs"]
mod tests;
