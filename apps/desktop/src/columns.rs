//! The column commands: a lane is created, renamed, reordered and deleted by
//! the person using it.

use devpit_core::Store;
use devpit_rpc::{Board, ColumnDeleted, ErrorCode, RpcError, StepKind};

use crate::board::board_get;

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

#[tauri::command]
#[specta::specta]
pub fn column_create(project_id: String, name: String) -> Result<Board, RpcError> {
    let store = store()?;
    let position = store.columns(&project_id)?.len() as i64;
    store.create_column(&project_id, &name, position)?;
    board_get(project_id)
}

#[tauri::command]
#[specta::specta]
pub fn column_rename(
    project_id: String,
    column_id: String,
    name: String,
) -> Result<Board, RpcError> {
    store()?.rename_column(&column_id, &name)?;
    board_get(project_id)
}

#[tauri::command]
#[specta::specta]
pub fn column_reorder(project_id: String, ids: Vec<String>) -> Result<Board, RpcError> {
    store()?.reorder_columns(&project_id, &ids)?;
    board_get(project_id)
}

/// What deleting a lane does with the cards that point at it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Deleting {
    /// Cards on the board are in it and nobody said where they go.
    Refused(u32),
    /// Every card, archived ones too, moves here before the lane goes.
    MovingTo(String),
    /// Nothing points at it.
    Plain,
}

/// Where a deleted lane's cards go, or the refusal that asks.
pub(crate) fn where_cards_go(
    lanes: &[String],
    column_id: &str,
    move_to: Option<&str>,
    in_the_way: u32,
) -> Result<Deleting, RpcError> {
    if !lanes.iter().any(|lane| lane == column_id) {
        return Err(RpcError::new(ErrorCode::NotFound, "no such column"));
    }
    match move_to {
        Some(to) if to == column_id => Err(RpcError::new(
            ErrorCode::Invalid,
            "cards cannot move into the lane being deleted",
        )),
        Some(to) if !lanes.iter().any(|lane| lane == to) => Err(RpcError::new(
            ErrorCode::Invalid,
            "that lane is not on this board",
        )),
        Some(to) => Ok(Deleting::MovingTo(to.to_owned())),
        None if in_the_way > 0 => Ok(Deleting::Refused(in_the_way)),
        // Archived cards still point at the lane and RESTRICT counts them: they
        // go where a restore looks first.
        None => Ok(lanes
            .iter()
            .find(|lane| *lane != column_id)
            .map_or(Deleting::Plain, |lane| Deleting::MovingTo(lane.clone()))),
    }
}

/// `column.delete` — refuses while cards are in it and says how many, or moves
/// them to `move_to` first.
///
/// Refusing is the answer, but a refusal without the number leaves the screen
/// asking a question it cannot phrase.
#[tauri::command]
#[specta::specta]
pub fn column_delete(
    project_id: String,
    column_id: String,
    move_to: Option<String>,
) -> Result<ColumnDeleted, RpcError> {
    let store = store()?;
    let lanes: Vec<String> = store
        .columns(&project_id)?
        .into_iter()
        .map(|lane| lane.id)
        .collect();
    let in_the_way = store
        .cards(&project_id)?
        .into_iter()
        .filter(|card| card.column_id == column_id)
        .count() as u32;

    match where_cards_go(&lanes, &column_id, move_to.as_deref(), in_the_way)? {
        Deleting::Refused(count) => {
            return Ok(ColumnDeleted {
                deleted: false,
                cards_in_the_way: count,
            })
        }
        Deleting::MovingTo(to) => store.delete_column_moving_cards(&column_id, &to)?,
        Deleting::Plain => store.delete_column(&column_id)?,
    }
    Ok(ColumnDeleted {
        deleted: true,
        cards_in_the_way: 0,
    })
}

#[tauri::command]
#[specta::specta]
pub fn step_create(
    project_id: String,
    kind: String,
    name: String,
    config: String,
    irreversible: bool,
) -> Result<Board, RpcError> {
    let store = store()?;
    // Refused now, while the person is still looking at what they typed.
    if let Some(why) = refused(&kind, &config) {
        return Err(RpcError::new(ErrorCode::Invalid, why));
    }
    store.create_step(&project_id, &kind, &name, &config, irreversible)?;
    board_get(project_id)
}

/// `column.set_step` — what this lane runs, or nothing.
#[tauri::command]
#[specta::specta]
pub fn column_set_step(
    project_id: String,
    column_id: String,
    step_id: Option<String>,
) -> Result<Board, RpcError> {
    store()?.set_column_step(&column_id, step_id.as_deref())?;
    board_get(project_id)
}

/// `column.set_flow` — where a pass goes, and how much the lane decides.
///
/// Refused rather than corrected when the two disagree: a lane cannot send a
/// card to itself, and it cannot advance to a lane that is not on this board.
#[tauri::command]
#[specta::specta]
pub fn column_set_flow(
    project_id: String,
    column_id: String,
    on_pass: Option<String>,
    autonomy: String,
) -> Result<Board, RpcError> {
    let store = store()?;

    // Read back rather than trusted: the word keys a rule that moves somebody's
    // card, and one this build does not know must not reach the column.
    let autonomy = crate::advancing::Autonomy::parse(&autonomy);

    if let Some(to) = on_pass.as_deref() {
        if to == column_id {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "a lane cannot send a card to itself",
            ));
        }
        let here = store.columns(&project_id)?;
        if !here.iter().any(|column| column.id == to) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "that lane is not on this board",
            ));
        }

        // Asked of the board this change *would* make, not the one it is:
        // two lanes approving into each other is a card that moves for ever,
        // spending money on every hop with nobody watching. Refused here,
        // while somebody is looking at what they chose.
        let mut flow: std::collections::HashMap<String, String> = here
            .iter()
            .filter(|column| column.id != column_id)
            .filter_map(|column| column.on_pass.clone().map(|goes| (column.id.clone(), goes)))
            .collect();
        flow.insert(column_id.clone(), to.to_owned());
        if crate::cycles::loops(&flow, &column_id) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "that would send a card round in a circle",
            ));
        }
    }

    if !store.set_column_flow(&column_id, on_pass.as_deref(), autonomy.stored())? {
        return Err(RpcError::new(ErrorCode::NotFound, "no such column"));
    }
    board_get(project_id)
}

/// Why a step cannot be saved, or nothing.
///
/// The catalogues are read here and the rule itself takes them as arguments,
/// so the rule is testable on a machine that has neither.
fn refused(kind: &str, config: &str) -> Option<String> {
    let kind = match kind {
        "agent" => StepKind::Agent,
        "command" => StepKind::Command,
        "session" => StepKind::Session,
        _ => return Some(format!("there is no kind of step called `{kind}`")),
    };
    let agents: Vec<String> = devpit_agentcli::read_every_agent(&devpit_agentcli::seed_sources())
        .agents
        .into_iter()
        .map(|one| one.name)
        .collect();
    let profiles: Vec<String> = store()
        .and_then(|store| crate::agent_profiles::all(&store))
        .unwrap_or_default()
        .into_iter()
        .filter(|one| one.mine)
        .map(|one| one.id)
        .collect();
    crate::steps::recipe::refuse(kind, config, &agents, &profiles)
}

#[cfg(test)]
#[path = "columns_tests.rs"]
mod tests;
