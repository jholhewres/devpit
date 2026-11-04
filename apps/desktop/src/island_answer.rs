//! The question a terminal session is stopped on, read and answered from the
//! island.
//!
//! What the hook said is what the island shows first — the question and its
//! choices, the moment it is asked. Answering goes by the screen, as from the
//! Sessions panel: the choices as the terminal draws them, the cursor where it
//! is, and a pick pressed only while the screen still shows the question the
//! person saw. It is the person answering, in their own terminal.

use devpit_rpc::{ErrorCode, PendingPrompt, RpcError};

/// The terminal a session of the island runs in, whichever holds it.
fn target_of(session_id: &str) -> Result<String, RpcError> {
    let (project, pane) = crate::island_feed::registry()
        .lock()
        .ok()
        .and_then(|sessions| {
            let one = sessions.get(session_id)?;
            Some((one.project_id.clone()?, one.pane_id.clone()?))
        })
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                "that session is in no terminal of devpit",
            )
        })?;
    crate::pane_io::target_of(&project, &pane)
        .map_err(|_| RpcError::new(ErrorCode::NotFound, "its terminal is closed"))
}

/// `island.prompt` — the question a session's terminal shows, if it shows one.
#[tauri::command]
#[specta::specta]
pub async fn island_prompt(session_id: String) -> Result<Option<PendingPrompt>, RpcError> {
    crate::off_main::blocking(move || {
        let target = target_of(&session_id)?;
        Ok(crate::pane_io::screen(&target).and_then(|shown| crate::live_prompt::pending(&shown)))
    })
    .await
}

/// The most a reply from the island may be, in characters: as from the
/// Sessions panel.
const LONGEST_REPLY: usize = 4000;

/// `island.reply` — the person's words, typed into a session's terminal as
/// theirs: a follow-up, a "go on", what the session asked for in its own
/// words. Never into a terminal whose shell is in front, where it would run as
/// a command.
#[tauri::command]
#[specta::specta]
pub async fn island_reply(session_id: String, text: String) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let text = text.trim();
        if text.is_empty() || text.chars().count() > LONGEST_REPLY {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "a reply is between 1 and 4000 characters",
            ));
        }
        let target = target_of(&session_id)?;
        if crate::pane_io::shell_in_front(&target)? {
            return Err(RpcError::new(
                ErrorCode::Conflict,
                "the agent is not in front in that terminal — open it to see why",
            ));
        }
        crate::pane_io::paste(&target, text, true)
    })
    .await
}

/// `island.typing` — the island takes the keyboard while a field of it is
/// being typed in, and gives it back after. Otherwise it never takes focus:
/// a window above everything that took the keyboard on every hover would be
/// a trap for whatever the person was typing elsewhere.
#[tauri::command]
#[specta::specta]
pub async fn island_typing(app: tauri::AppHandle, on: bool) -> Result<(), RpcError> {
    use tauri::Manager as _;
    let Some(window) = app.get_webview_window(crate::island::ISLAND) else {
        return Ok(());
    };
    let _ = window.set_focusable(on);
    if on {
        let _ = window.set_focus();
    }
    Ok(())
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
