//! The question a terminal session is stopped on, read and answered from the
//! island.
//!
//! What the hook said is what the island shows first — the question and its
//! choices, the moment it is asked. Answering goes by the screen, as from the
//! Sessions panel: the choices as the terminal draws them, the cursor where it
//! is, and a pick pressed only while the screen still shows the question the
//! person saw. It is the person answering, in their own terminal.

use devpit_rpc::{ErrorCode, PendingPrompt, RpcError};

/// The terminal a session of the island runs in, as tmux points at it.
fn target_of(session_id: &str) -> Result<String, RpcError> {
    let pane = crate::island_feed::registry()
        .lock()
        .ok()
        .and_then(|sessions| sessions.get(session_id)?.pane_id.clone())
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                "that session is in no terminal of devpit",
            )
        })?;
    let session = crate::sessions::tmux_server()?
        .session_of_window(&pane)
        .map_err(|err| RpcError::internal(err.to_string()))?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "its terminal is closed"))?;
    Ok(devpit_tmux::Server::target(&session, &pane))
}

/// `island.prompt` — the question a session's terminal shows, if it shows one.
#[tauri::command]
#[specta::specta]
pub async fn island_prompt(session_id: String) -> Result<Option<PendingPrompt>, RpcError> {
    crate::off_main::blocking(move || {
        let target = target_of(&session_id)?;
        Ok(crate::live_sessions::screen_of(&target)
            .and_then(|shown| crate::live_prompt::pending(&shown)))
    })
    .await
}

/// `island.answer` — the person's pick on that question, or Escape when
/// `choice` is absent.
#[tauri::command]
#[specta::specta]
pub async fn island_answer(
    session_id: String,
    seen: PendingPrompt,
    choice: Option<u32>,
) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        crate::live_answer::answer_in(&target_of(&session_id)?, &seen, choice)
    })
    .await
}
