//! A leaf's screen: scrolled through its history, or drawn again whole.
//! Apart from `panes.rs`, which attaches and feeds the pty.

use devpit_rpc::RpcError;

/// `session.scroll` — a wheel over a leaf: its history, or its program's
/// arrow keys when one holds the screen. Zero goes back to the live screen.
///
/// Asked of tmux rather than sent through the pty: the client draws on the
/// alternate screen, where a wheel is arrow keys and the shell echoes them.
/// Answers whether tmux took it; a ptyd pane's history is the window's own.
#[tauri::command]
#[specta::specta]
pub async fn session_scroll(
    project_id: String,
    pane_id: String,
    lines: i32,
) -> Result<bool, RpcError> {
    crate::off_main::blocking(move || session_scroll_now(project_id, pane_id, lines)).await
}

/// [`session_scroll`], on the calling thread.
pub(crate) fn session_scroll_now(
    project_id: String,
    pane_id: String,
    lines: i32,
) -> Result<bool, RpcError> {
    let target = crate::pane_io::target_of(&project_id, &pane_id)?;
    crate::pane_io::scroll(&target, lines)
}

/// `session.redraw` — this leaf's screen drawn again, whole.
#[tauri::command]
#[specta::specta]
pub async fn session_redraw(
    app: tauri::AppHandle,
    project_id: String,
    pane_id: String,
) -> Result<bool, RpcError> {
    crate::off_main::blocking(move || session_redraw_now(&app, project_id, pane_id)).await
}

/// [`session_redraw`], on the calling thread.
pub(crate) fn session_redraw_now(
    app: &tauri::AppHandle,
    project_id: String,
    pane_id: String,
) -> Result<bool, RpcError> {
    let target = crate::pane_io::target_of(&project_id, &pane_id)?;
    crate::pane_io::redraw(app, &target)
}
