//! A side question to a conversation (`/btw`), in a process of its own.

use std::path::PathBuf;

use devpit_agentcli::aside::{ask, Aside};
use devpit_agentcli::head::{head_path, read_head};
use devpit_rpc::{ErrorCode, RpcError};

/// The longest question sent: a side question is a sentence or two.
const MOST: usize = 4000;

/// `chat.aside` — a side question (`/btw`), answered from the conversation
/// forked: the turn running goes on, and its history never holds it.
#[tauri::command]
#[specta::specta]
pub async fn chat_aside(
    project_id: String,
    conversation_id: String,
    question: String,
) -> Result<String, RpcError> {
    crate::off_main::blocking(move || {
        let question: String = question.trim().chars().take(MOST).collect();
        if question.is_empty() {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "ask something after /btw",
            ));
        }
        let sessions = crate::projects::project_home(&project_id)?.sessions();
        let head = read_head(&head_path(&sessions, &conversation_id)).ok_or_else(|| {
            RpcError::new(ErrorCode::NotFound, "this conversation has not started yet")
        })?;
        let (profile, path) = crate::chat_turn::spawnable(&head.profile)?;
        if profile.driver != "claude" {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "/btw is answered by Claude Code only",
            ));
        }
        let env = devpit_agentcli::running::runner(&profile).env;
        let cwd = match head.cwd.as_deref() {
            Some(cwd) => PathBuf::from(cwd),
            None => crate::roots::root_of(&project_id, None)?,
        };
        ask(&Aside {
            command: &path,
            env: &env,
            cwd: &cwd,
            session_id: head.session_id.as_deref(),
            model: head.model.as_deref(),
            question: &question,
        })
        .map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}
