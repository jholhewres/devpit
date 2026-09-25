//! The person's pick on the question a session is stopped on, pressed in its
//! terminal.
//!
//! Only when the question is still the one the person saw: every word of it
//! and every choice, since two permission prompts in a row both open on
//! "1. Yes". And Enter only once the cursor is seen on the choice — keys that
//! arrive faster than the program reads them must not take the wrong one.

use std::time::Duration;

use devpit_rpc::{ErrorCode, PendingPrompt, RpcError};
use devpit_tmux::Key;

use crate::live_sessions::{screen_of, terminal_of};

/// How long the cursor is given to reach the choice before nothing is taken.
const CURSOR_TRIES: u32 = 20;
const CURSOR_WAIT: Duration = Duration::from_millis(25);

/// The arrows that move a prompt's cursor from `cursor` to `choice`.
pub(crate) fn keys_for(cursor: u32, choice: u32) -> Vec<Key> {
    if choice >= cursor {
        vec![Key::Down; (choice - cursor) as usize]
    } else {
        vec![Key::Up; (cursor - choice) as usize]
    }
}

/// Whether what is on screen `now` is still the question the person `seen`,
/// and `choice` one of its choices.
pub(crate) fn still_asked(
    now: Option<&PendingPrompt>,
    seen: &PendingPrompt,
    choice: Option<u32>,
) -> Result<(), RpcError> {
    let Some(now) = now else {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "it is not waiting on a question any more",
        ));
    };
    if now != seen {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "that question has changed — look again",
        ));
    }
    if choice.is_some_and(|at| at as usize >= now.options.len()) {
        return Err(RpcError::new(ErrorCode::Invalid, "no such choice"));
    }
    Ok(())
}

/// Whether the cursor sits on `choice` of the same question.
fn on_choice(now: Option<&PendingPrompt>, seen: &PendingPrompt, choice: u32) -> bool {
    now.is_some_and(|now| {
        now.cursor == choice && now.question == seen.question && now.options == seen.options
    })
}

/// `orchestrator.answer` — the person's pick on the question a session is
/// stopped on: arrows to the choice and Enter, or Escape when `choice` is
/// absent. The window's alone, like replying.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_answer(
    profile_id: String,
    name: String,
    seen: PendingPrompt,
    choice: Option<u32>,
) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let target = terminal_of(&profile_id, &name)?;
        let server = crate::sessions::tmux_server()?;
        let read = || screen_of(&target).and_then(|shown| crate::live_prompt::pending(&shown));
        let press = |key: Key| {
            server
                .press(&target, &[key])
                .map_err(|err| RpcError::internal(err.to_string()))
        };
        still_asked(read().as_ref(), &seen, choice)?;
        let Some(at) = choice else {
            return press(Key::Escape);
        };
        for key in keys_for(seen.cursor, at) {
            press(key)?;
        }
        for _ in 0..CURSOR_TRIES {
            if on_choice(read().as_ref(), &seen, at) {
                return press(Key::Enter);
            }
            std::thread::sleep(CURSOR_WAIT);
        }
        Err(RpcError::new(
            ErrorCode::Conflict,
            "the cursor did not reach that choice — nothing was taken",
        ))
    })
    .await
}

#[cfg(test)]
#[path = "live_answer_tests.rs"]
mod tests;
