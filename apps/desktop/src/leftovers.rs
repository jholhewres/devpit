//! What a project's tmux session holds that no tab points at any more.
//!
//! tmux outlives the app, and so does whatever went wrong while it was closed:
//! a shell that exited took its window and left its tap, a crash between
//! making a window and saving the tree left a window no tab holds, and a
//! forgotten project left its whole session running. Nothing on screen can
//! reach any of those again, so they are tidied where the session is reached.

use std::collections::HashSet;

use devpit_core::Store;
use devpit_rpc::RpcError;
use tauri::Manager;

use crate::sessions::{decode, store, tmux_server, SessionState};

/// Tidies a project's session as it is ensured, under the project's lock.
/// `had` is the session's windows before anything was made for this call.
pub(crate) fn tidy(state: &SessionState, project_id: &str, had: &[String]) {
    let Ok(server) = tmux_server() else {
        return;
    };
    state.taps.sweep(&server, project_id, had);
    let Ok(store) = store() else {
        return;
    };
    let strays = unheld(had, &held(&store, project_id));
    if strays.is_empty() {
        return;
    }
    let session = devpit_tmux::Server::session_name(project_id);
    let panes = server.running(&session).unwrap_or_default();
    let ttys: Vec<String> = panes.iter().map(|pane| pane.tty.clone()).collect();
    let fronts = devpit_pty::looking(&ttys);
    for leaf in strays {
        let Some(pane) = panes.iter().find(|pane| pane.leaf_id == leaf) else {
            continue;
        };
        let at_prompt =
            devpit_pty::front_on(&fronts, &pane.tty).is_some_and(devpit_pty::at_a_prompt);
        let under = server
            .pane_pid(&session, &leaf)
            .and_then(|shell| devpit_pty::session_members(&[shell]))
            .map(|members| members.len());
        if !closable(at_prompt, under) {
            eprintln!("devpit: {leaf} of {project_id} is in no tab, and left running: something works in it");
            continue;
        }
        let target = devpit_tmux::Server::target(&session, &leaf);
        state.taps.forget(&server, &leaf, &target);
        let _ = server.kill_window(&session, &leaf);
        crate::blocks::closed(state.app(), project_id, &leaf);
        let _ = store.forget_pane_agent(&leaf);
    }
}

/// Every leaf any of the project's tabs holds.
fn held(store: &Store, project_id: &str) -> HashSet<String> {
    let tabs = store.pane_layout_tabs(project_id).unwrap_or_default();
    tabs.iter()
        .filter_map(|tab| store.pane_layout(project_id, tab).ok().flatten())
        .filter_map(|(tree, focused)| decode(project_id, &tree, &focused).ok())
        .flat_map(|layout| {
            let leaves: Vec<String> = layout
                .tree
                .leaves()
                .into_iter()
                .map(|(leaf, _)| leaf.to_owned())
                .collect();
            leaves
        })
        .collect()
}

/// The windows no tab holds.
pub(crate) fn unheld(windows: &[String], held: &HashSet<String>) -> Vec<String> {
    windows
        .iter()
        .filter(|window| !held.contains(*window))
        .cloned()
        .collect()
}

/// Whether a window no tab holds may go: only a shell at its prompt with
/// nothing under it. Anything working is somebody's, and is only reported.
/// `under` is `None` where the process table cannot be read.
pub(crate) fn closable(at_prompt: bool, under: Option<usize>) -> bool {
    at_prompt && under.unwrap_or(0) == 0
}

/// Ends every terminal of a project that is being forgotten: its tabs, then
/// whatever else its session held, then the session itself.
pub(crate) fn end_project(app: &tauri::AppHandle, project_id: &str) -> Result<(), RpcError> {
    if !devpit_tmux::Server::available() {
        return Ok(());
    }
    // Only a project the store knows: session names are sanitised, so an id
    // from the window could otherwise name another project's session.
    let store = store()?;
    if store.project(project_id)?.is_none() {
        return Ok(());
    }
    let state = app.state::<SessionState>();
    let mut closed = Vec::new();
    for tab in store.pane_layout_tabs(project_id)? {
        closed.extend(crate::arranging::close_tab(&state, project_id, &tab)?);
    }
    crate::card_activity::panes_closed(app, &closed);

    let lock = state.project_lock(project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let server = tmux_server()?;
    let session = devpit_tmux::Server::session_name(project_id);
    if !server.has_session(&session).unwrap_or(false) {
        return Ok(());
    }
    let windows = server.list_windows(&session).unwrap_or_default();
    let leaves: Vec<&str> = windows.iter().map(String::as_str).collect();
    for leaf in &leaves {
        let target = devpit_tmux::Server::target(&session, leaf);
        state.taps.forget(&server, leaf, &target);
    }
    crate::tap::stop_whatever_runs(&server, &session, &leaves);
    server
        .kill_session(&session)
        .map_err(crate::sessions::tmux_err)
}

#[cfg(test)]
#[path = "leftovers_tests.rs"]
mod tests;
