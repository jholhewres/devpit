//! Changing the shape of the pane tree.
//!
//! Apart from [`crate::sessions`], which opens a session and reads it: a split,
//! a close, a rename and a dragged boundary all rewrite the tree and persist
//! it, and they share the project lock that keeps two windows from arranging
//! the same project at once.

use devpit_rpc::{ErrorCode, LayoutNode, RpcError, SessionLayout, SplitDirection};
use tauri::State;
use ulid::Ulid;

use crate::sessions::{layout_of, locate_cwd, persist, store, tmux_err, tmux_server, SessionState};

/// `session.split` — a new tmux window and a split node in the tree.
#[tauri::command]
#[specta::specta]
pub fn session_split(
    state: State<SessionState>,
    project_id: String,
    tab_id: String,
    leaf_id: String,
    direction: SplitDirection,
    worktree_id: Option<String>,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let cwd = locate_cwd(&project_id, worktree_id.as_deref())?;
    let server = tmux_server()?;
    let session = devpit_tmux::Server::session_name(&project_id);

    let current = layout_of(&project_id, &tab_id)?;
    if !current.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        ));
    }

    let new_id = format!("leaf_{}", Ulid::generate());
    server
        .new_window(&session, &new_id, &cwd)
        .map_err(tmux_err)?;
    let new_leaf = LayoutNode::leaf(
        new_id.clone(),
        devpit_tmux::Server::target(&session, &new_id),
    );
    let tree = current
        .tree
        .split_leaf(&leaf_id, direction, new_leaf)
        .ok_or_else(|| RpcError::internal("the leaf vanished while splitting"))?;

    let layout = SessionLayout {
        project_id,
        focused_id: new_id,
        tree,
    };
    persist(&store()?, &tab_id, &layout)?;
    Ok(layout)
}

/// `session.close_leaf` — the pane goes, and its tmux window with it.
///
/// The hole this fills: `session.split` could only ever add. A tree that only
/// grows is a leak wearing a layout's clothes.
///
/// Refuses the last pane. A session with no pane is not a layout, and the
/// refusal says so rather than persisting an empty tree the screen cannot draw.
#[tauri::command]
#[specta::specta]
pub fn session_close_leaf(
    app: tauri::AppHandle,
    state: State<SessionState>,
    project_id: String,
    tab_id: String,
    leaf_id: String,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let current = layout_of(&project_id, &tab_id)?;
    if !current.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        ));
    }
    let tree = current.tree.close_leaf(&leaf_id).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Conflict,
            "that is the only pane left — close the project instead",
        )
    })?;

    // The window goes after the tree is known to be closable, so a refusal
    // never leaves a session whose layout and tmux disagree.
    let session = devpit_tmux::Server::session_name(&project_id);
    let server = tmux_server()?;
    // Stop listening before the window goes: the tap holds a fifo and a
    // thread, and a pane that no longer exists has nothing left to say.
    state.taps.forget(
        &server,
        &leaf_id,
        &devpit_tmux::Server::target(&session, &leaf_id),
    );
    crate::tap::stop_whatever_runs(&server, &session, &leaf_id);
    server.kill_window(&session, &leaf_id).map_err(tmux_err)?;
    // A pane closed on purpose is not one to start an agent in again.
    let _ = store()?.forget_pane_agent(&leaf_id);
    crate::card_activity::panes_closed(&app, std::slice::from_ref(&leaf_id));

    // Focus follows the tree when it pointed at what just left.
    let focused_id = if current.focused_id == leaf_id {
        tree.first_leaf_id().to_owned()
    } else {
        current.focused_id
    };
    let layout = SessionLayout {
        project_id,
        focused_id,
        tree,
    };
    persist(&store()?, &tab_id, &layout)?;
    Ok(layout)
}

/// `session.rename_leaf` — the name the person gave this pane.
///
/// An empty name clears it, which is how a pane goes back to showing what the
/// program running in it calls itself. The person's name always wins over the
/// program's: a title escape arriving later must not undo a rename.
#[tauri::command]
#[specta::specta]
pub fn session_rename_leaf(
    state: State<SessionState>,
    project_id: String,
    tab_id: String,
    leaf_id: String,
    name: String,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let current = layout_of(&project_id, &tab_id)?;
    let tree = current.tree.rename_leaf(&leaf_id, &name).ok_or_else(|| {
        RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this project's layout",
        )
    })?;
    let layout = SessionLayout {
        project_id,
        focused_id: current.focused_id,
        tree,
    };
    persist(&store()?, &tab_id, &layout)?;
    Ok(layout)
}

/// `session.set_ratio` — where a boundary was dragged to.
///
/// Persisted because Orca's rule is the right one: boundaries stay where you
/// put them, and resizing the window does not shuffle a layout someone
/// arranged. The tree clamps, so neither side can be dragged out of reach.
#[tauri::command]
#[specta::specta]
pub fn session_set_ratio(
    state: State<SessionState>,
    project_id: String,
    tab_id: String,
    split_id: String,
    ratio: f64,
) -> Result<SessionLayout, RpcError> {
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let current = layout_of(&project_id, &tab_id)?;
    let tree = current.tree.set_ratio(&split_id, ratio).ok_or_else(|| {
        RpcError::new(
            ErrorCode::NotFound,
            "no such boundary in this project's layout",
        )
    })?;
    let layout = SessionLayout {
        project_id,
        focused_id: current.focused_id,
        tree,
    };
    persist(&store()?, &tab_id, &layout)?;
    Ok(layout)
}

/// `session.close_tab` — the tab goes, and every window in its tree with it.
///
/// A tab owns a tree, so closing one is not closing a leaf: leaving the rest
/// running would leave shells nothing can reach again, which is the shape the
/// scrollback and the layout both keyed on.
#[tauri::command]
#[specta::specta]
pub fn session_close_tab(
    app: tauri::AppHandle,
    state: State<SessionState>,
    project_id: String,
    tab_id: String,
) -> Result<(), RpcError> {
    let closed = close_tab(&state, &project_id, &tab_id)?;
    crate::card_activity::panes_closed(&app, &closed);
    Ok(())
}

/// The body of `session.close_tab`, for a card that takes its own tab with it.
/// Answers the leaves it closed.
pub(crate) fn close_tab(
    state: &SessionState,
    project_id: &str,
    tab_id: &str,
) -> Result<Vec<String>, RpcError> {
    let lock = state.project_lock(project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    // A tab with no tree is already closed. Saying so is not an error: the
    // window asks on every close, including ones that never opened a session.
    let Ok(layout) = layout_of(project_id, tab_id) else {
        return Ok(Vec::new());
    };
    let closed: Vec<String> = layout
        .tree
        .leaves()
        .into_iter()
        .map(|(leaf, _)| leaf.to_owned())
        .collect();
    let server = tmux_server()?;
    let session = devpit_tmux::Server::session_name(project_id);
    for (leaf_id, _) in layout.tree.leaves() {
        let target = devpit_tmux::Server::target(&session, leaf_id);
        state.taps.forget(&server, leaf_id, &target);
        crate::tap::stop_whatever_runs(&server, &session, leaf_id);
        let _ = server.kill_window(&session, leaf_id);
        let _ = store()?.forget_pane_agent(leaf_id);
    }
    store()?.forget_pane_layout(project_id, tab_id)?;
    Ok(closed)
}
