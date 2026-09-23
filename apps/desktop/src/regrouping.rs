//! Moving panes between tabs: a whole tab into another's split, or one pane
//! out into a tab of its own.
//!
//! Neither touches tmux. A leaf names its window, not its tab, so a move is a
//! rewrite of two trees in one transaction — and the processes in the panes
//! never notice. That is also why this is apart from [`crate::arranging`],
//! whose close paths do kill windows.

use devpit_core::PaneLayoutWrite;
use devpit_rpc::{ErrorCode, RpcError, SessionLayout, SplitDirection};
use tauri::State;
use ulid::Ulid;

use crate::sessions::{encode, is_card_tab, layout_of, store, SessionState};

/// Why `from` cannot be joined into `into`, when it cannot.
///
/// A card's tab is how the card finds its panes, so a pane moved into one
/// would ring a card it has nothing to do with, and a card's tab joined away
/// would leave the card with no terminal.
pub(crate) fn refuse_join(from: &str, into: &str) -> Option<&'static str> {
    if from == into {
        return Some("a tab cannot be joined into itself");
    }
    if is_card_tab(from) || is_card_tab(into) {
        return Some("a card's terminal keeps a tab of its own");
    }
    None
}

/// `session.join_tabs` — every pane of `from_tab_id` beside `into_tab_id`'s,
/// and `from_tab_id` gone. Answers the joined layout.
#[tauri::command]
#[specta::specta]
pub fn session_join_tabs(
    state: State<SessionState>,
    project_id: String,
    from_tab_id: String,
    into_tab_id: String,
    direction: SplitDirection,
) -> Result<SessionLayout, RpcError> {
    if let Some(why) = refuse_join(&from_tab_id, &into_tab_id) {
        return Err(RpcError::new(ErrorCode::Conflict, why));
    }
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let from = layout_of(&project_id, &from_tab_id)?;
    let into = layout_of(&project_id, &into_tab_id)?;
    let layout = SessionLayout {
        project_id,
        // What moved is what the person asked about, so it is what has focus.
        focused_id: from.focused_id,
        tree: into
            .tree
            .joined(from.tree, direction, format!("sp_{}", Ulid::generate())),
    };
    let tree = encode(&layout)?;
    let write = PaneLayoutWrite {
        tab_id: &into_tab_id,
        tree: &tree,
        focused_id: &layout.focused_id,
    };
    store()?.regroup_pane_layouts(&layout.project_id, &[write], &[&from_tab_id])?;
    Ok(layout)
}

/// `session.separate_leaf` — one pane out of `tab_id` into `new_tab_id`, a
/// tab the window is about to open. Answers what is left in `tab_id`.
#[tauri::command]
#[specta::specta]
pub fn session_separate_leaf(
    state: State<SessionState>,
    project_id: String,
    tab_id: String,
    leaf_id: String,
    new_tab_id: String,
) -> Result<SessionLayout, RpcError> {
    if is_card_tab(&new_tab_id) {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "a pane cannot be moved into a card's tab",
        ));
    }
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;

    let store = store()?;
    // Moving into a tab that has a tree would drop that tree on the floor.
    if store.pane_layout(&project_id, &new_tab_id)?.is_some() {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "that tab already has panes",
        ));
    }
    let current = layout_of(&project_id, &tab_id)?;
    if !current.tree.contains_leaf(&leaf_id) {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that pane is not in this tab",
        ));
    }
    let (rest, leaf) = current.tree.detached(&leaf_id).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Conflict,
            "that is the only pane in this tab — it already has a tab of its own",
        )
    })?;

    let focused_id = if current.focused_id == leaf_id {
        rest.first_leaf_id().to_owned()
    } else {
        current.focused_id
    };
    let left = SessionLayout {
        project_id: project_id.clone(),
        focused_id,
        tree: rest,
    };
    let moved = SessionLayout {
        project_id,
        focused_id: leaf_id,
        tree: leaf,
    };
    let (left_tree, moved_tree) = (encode(&left)?, encode(&moved)?);
    let writes = [
        PaneLayoutWrite {
            tab_id: &tab_id,
            tree: &left_tree,
            focused_id: &left.focused_id,
        },
        PaneLayoutWrite {
            tab_id: &new_tab_id,
            tree: &moved_tree,
            focused_id: &moved.focused_id,
        },
    ];
    store.regroup_pane_layouts(&left.project_id, &writes, &[])?;
    Ok(left)
}

#[cfg(test)]
#[path = "regrouping_tests.rs"]
mod tests;
