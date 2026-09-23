//! A leaf's screen, asked of tmux: scrolled through its history, or drawn
//! again whole. Apart from `panes.rs`, which attaches and feeds the pty.

use devpit_rpc::RpcError;

/// `session.scroll` — a wheel over a leaf: its history, or its program's
/// arrow keys when one holds the screen. Zero goes back to the live screen.
///
/// Asked of tmux rather than sent through the pty: the client draws on the
/// alternate screen, where a wheel is arrow keys and the shell echoes them.
#[tauri::command]
#[specta::specta]
pub async fn session_scroll(
    project_id: String,
    pane_id: String,
    lines: i32,
) -> Result<(), RpcError> {
    crate::off_main::blocking(move || session_scroll_now(project_id, pane_id, lines)).await
}

/// [`session_scroll`], on the calling thread.
pub(crate) fn session_scroll_now(
    project_id: String,
    pane_id: String,
    lines: i32,
) -> Result<(), RpcError> {
    crate::sessions::holding(&project_id, &pane_id)?;
    let session = devpit_tmux::Server::session_name(&project_id);
    crate::sessions::tmux_server()?
        .scroll(&devpit_tmux::Server::target(&session, &pane_id), lines)
        .map_err(|err| RpcError::internal(err.to_string()))
}

/// `session.redraw` — tmux draws this leaf's screen again, whole.
#[tauri::command]
#[specta::specta]
pub async fn session_redraw(project_id: String, pane_id: String) -> Result<bool, RpcError> {
    crate::off_main::blocking(move || session_redraw_now(project_id, pane_id)).await
}

/// [`session_redraw`], on the calling thread.
pub(crate) fn session_redraw_now(project_id: String, pane_id: String) -> Result<bool, RpcError> {
    crate::sessions::holding(&project_id, &pane_id)?;
    let session = devpit_tmux::Server::session_name(&project_id);
    crate::sessions::tmux_server()?
        .redraw(&session, &pane_id)
        .map_err(|err| RpcError::internal(err.to_string()))
}
