//! A tab's first pane, and a card's session brought into its own tab.
//!
//! Apart from [`crate::sessions`], which loads and keeps the layouts: what
//! is here makes a pane and types into it, whichever holds it.

use std::path::Path;

use devpit_core::Store;
use devpit_rpc::{CardTerminal, ErrorCode, LayoutNode, RpcError, SessionLayout};
use tauri::State;
use ulid::Ulid;

use crate::sessions::{ensure_at, persist, store, tab_for_card, SessionState};

/// A new tab's layout: one pane, made where every new pane is made
/// ([`crate::ptyd_link::new_pane`]) whatever the tab is for — a plain
/// terminal, an agent's or a card's — and kept.
pub(crate) fn first_layout(
    store: &Store,
    home: Option<&Path>,
    project_id: &str,
    tab_id: &str,
    cwd: &Path,
    in_tmux: impl FnOnce(&str) -> Result<String, RpcError>,
) -> Result<SessionLayout, RpcError> {
    let leaf_id = format!("leaf_{}", Ulid::generate());
    let target =
        crate::ptyd_link::new_pane(store, home, project_id, &leaf_id, cwd, || in_tmux(&leaf_id))?;
    let layout = SessionLayout {
        project_id: project_id.to_owned(),
        focused_id: leaf_id.clone(),
        tree: LayoutNode::leaf(leaf_id, target),
    };
    persist(store, tab_id, &layout)?;
    Ok(layout)
}

/// `terminal.attach_agent` — brings a card's background session into the
/// card's own terminal, in its checkout.
///
/// The session that was there keeps running detached; attaching takes up the
/// screen, not the work. The line is typed into the tab's focused pane, so a
/// pane running something else is refused rather than typed over — and this is
/// only ever an action of the interface, never a side effect of a drag.
#[tauri::command]
#[specta::specta]
pub async fn terminal_attach_agent(
    state: State<'_, SessionState>,
    project_id: String,
    card_id: String,
) -> Result<CardTerminal, RpcError> {
    let (project, wanted) = (project_id.clone(), card_id.clone());
    let (short_id, checkout, runner) = tauri::async_runtime::spawn_blocking(move || {
        let store = store()?;
        if store.live_card_project(&wanted)?.as_deref() != Some(project.as_str()) {
            return Err(RpcError::new(
                ErrorCode::NotFound,
                "no such card in this project",
            ));
        }
        let link = store
            .session_link(&wanted)?
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "this card has no session yet"))?;
        let checkout = crate::checkout::checkout_of(&store, &wanted, |_| {})
            .map_err(|why| RpcError::new(ErrorCode::Internal, why))?;
        // Started under a profile, attached under the same one: the binary and
        // the account are the session's, not this moment's default.
        let runner = link
            .profile_id
            .as_deref()
            .map(|id| crate::agent_profiles::runner_for(&store, id))
            .transpose()
            .map_err(|why| RpcError::new(ErrorCode::NotFound, why))?;
        Ok::<_, RpcError>((link.short_id, checkout, runner))
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))??;

    // The card's own tab, made at its checkout if it is not there yet.
    let tab_id = tab_for_card(&card_id);
    let layout = ensure_at(&state, &project_id, &tab_id, &checkout)?;
    let target = crate::pane_io::target_in(&layout, &layout.focused_id);

    // Not under the project lock: a shell reaching its prompt takes seconds.
    let waiting = target.clone();
    let ready = tauri::async_runtime::spawn_blocking(move || crate::pane_io::settled(&waiting))
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?;
    let line = crate::attaching::attach_target(&checkout, &short_id, runner.as_ref(), &ready)?;

    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    crate::pane_io::send_line(&target, &line)?;
    Ok(CardTerminal {
        layout,
        tab_id,
        card_id,
    })
}

#[cfg(test)]
#[path = "tab_opening_tests.rs"]
mod tests;
