//! Going back to an earlier turn, as a new conversation.
//!
//! Forked rather than rewound in place: the original keeps every turn it had,
//! on devpit's side and on the CLI's. The fork copies devpit's messages up to
//! the chosen turn, and its head asks the next turn to run with
//! `--fork-session --resume-session-at`, so the CLI brings the same history cut
//! at the same place.

use std::path::Path;

use devpit_agentcli::head::{head_path, read_head, write_head, Head, Rewind};
use devpit_agentcli::store::{append, conversation_path, read};
use devpit_rpc::{ErrorCode, Message, Part, Role, RpcError};

/// Writes conversation `to` as `from` up to the end of `turn_id`.
pub(crate) fn rewind(
    sessions: &Path,
    from: &str,
    turn_id: &str,
    to: &str,
    now: f64,
) -> Result<(), RpcError> {
    let head = read_head(&head_path(sessions, from))
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such conversation"))?;
    let Some(at) = head
        .rewind
        .anchors
        .iter()
        .position(|anchor| anchor.turn_id == turn_id)
    else {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "that turn ran before devpit kept its place in the CLI's transcript",
        ));
    };
    let (messages, _) = read(&conversation_path(sessions, from));
    let Some(last) = messages
        .iter()
        .rposition(|message| message.turn_id.as_deref() == Some(turn_id))
    else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            "that turn is not in the conversation",
        ));
    };

    let file = conversation_path(sessions, to);
    if file.exists() {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "that conversation exists",
        ));
    }
    let kept = &messages[..=last];
    let internal = |err: std::io::Error| RpcError::internal(err.to_string());
    for message in kept {
        append(&file, message).map_err(internal)?;
    }
    let turn = kept
        .iter()
        .filter(|message| message.role == Role::User)
        .count() as u32;
    append(
        &file,
        &Message {
            id: format!("msg_{}", ulid::Ulid::generate()),
            turn_id: None,
            role: Role::System,
            parts: vec![Part::Rewound {
                from_conversation: from.to_owned(),
                turn,
            }],
            created_at: now,
            streaming: false,
        },
    )
    .map_err(internal)?;

    let anchor = head.rewind.anchors[at].clone();
    write_head(
        &head_path(sessions, to),
        &Head {
            created_at: now,
            session_id: Some(anchor.session_id),
            rewind: Rewind {
                fork_at: Some(anchor.uuid),
                anchors: head.rewind.anchors[..=at].to_vec(),
            },
            // Cost and cap come along: a fork is not a way to spend a budget twice.
            ..head
        },
    )
    .map_err(internal)
}

/// `chat.rewind` — a new conversation that goes on from an earlier turn.
///
/// Answers its id, which the window opens as a chat tab.
#[tauri::command]
#[specta::specta]
pub async fn chat_rewind(
    project_id: String,
    conversation_id: String,
    turn_id: String,
) -> Result<String, RpcError> {
    crate::off_main::blocking(move || chat_rewind_now(project_id, conversation_id, turn_id)).await
}

/// [`chat_rewind`], on the calling thread.
pub(crate) fn chat_rewind_now(
    project_id: String,
    conversation_id: String,
    turn_id: String,
) -> Result<String, RpcError> {
    use crate::adopting::plain;
    if !plain(&project_id) || !plain(&conversation_id) || !plain(&turn_id) {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a turn"));
    }
    let to = format!("conv_{}", ulid::Ulid::generate());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default();
    rewind(
        &crate::projects::project_home(&project_id)?.sessions(),
        &conversation_id,
        &turn_id,
        &to,
        now,
    )?;
    Ok(to)
}

#[cfg(test)]
#[path = "rewinding_tests.rs"]
mod tests;
