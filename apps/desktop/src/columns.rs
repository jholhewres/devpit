//! The column commands: a lane is created, renamed, reordered and deleted by
//! the person using it.

use quockpit_core::Store;
use quockpit_rpc::{Board, ColumnDeleted, RpcError};

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

/// `column.delete` — refuses while cards are in it, and says how many.
///
/// Refusing is the answer, but a refusal without the number leaves the screen
/// asking a question it cannot phrase.
#[tauri::command]
#[specta::specta]
pub fn column_delete(project_id: String, column_id: String) -> Result<ColumnDeleted, RpcError> {
    let store = store()?;
    let in_the_way = store
        .cards(&project_id)?
        .into_iter()
        .filter(|card| card.column_id == column_id)
        .count() as u32;

    if in_the_way > 0 {
        return Ok(ColumnDeleted {
            deleted: false,
            cards_in_the_way: in_the_way,
        });
    }

    store.delete_column(&column_id)?;
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
