//! What a person answered to a permission question, kept in the conversation.
//!
//! The question leaves the composer once it is answered, and until this it
//! left no trace: reopening a conversation showed a command that ran, or a
//! refusal the agent worked around, with nothing saying a person decided it.
//! Written as its own line in the transcript, so it survives a restart like
//! every other message does.

use devpit_agentcli::store::{append, conversation_path};
use devpit_rpc::{ErrorCode, Message, Part, Role, RpcError};

/// The message a receipt is, before it is written.
pub(crate) fn receipt(
    id: String,
    created_at: f64,
    tool: String,
    input: String,
    allowed: bool,
) -> Message {
    Message {
        id,
        turn_id: None,
        role: Role::System,
        parts: vec![Part::Receipt {
            tool,
            input,
            allowed,
        }],
        created_at,
        streaming: false,
    }
}

/// An id that names a file, so it may not be a path.
fn plain(id: &str) -> bool {
    !id.is_empty() && !id.contains(['/', '\\']) && !id.starts_with('.')
}

/// `chat.receipt` — records an answered question in the conversation.
#[tauri::command]
#[specta::specta]
pub fn chat_receipt(
    project_id: String,
    conversation_id: String,
    tool: String,
    input: String,
    allowed: bool,
) -> Result<Message, RpcError> {
    if !plain(&project_id) || !plain(&conversation_id) {
        return Err(RpcError::new(
            ErrorCode::Forbidden,
            "that is not a conversation",
        ));
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default();
    let message = receipt(
        format!("msg_{}", ulid::Ulid::generate()),
        now,
        tool,
        input,
        allowed,
    );
    append(
        &conversation_path(&crate::chat::home(), &project_id, &conversation_id),
        &message,
    )
    .map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_receipt_is_a_system_line_naming_the_decision() {
        let message = receipt(
            "m".into(),
            1.0,
            "Bash".into(),
            "{\"command\":\"cargo test\"}".into(),
            true,
        );
        assert!(matches!(message.role, Role::System));
        assert!(matches!(
            &message.parts[..],
            [Part::Receipt { tool, allowed: true, .. }] if tool == "Bash"
        ));
    }

    /// Written through the same store every turn uses, and read back by it:
    /// a receipt that does not survive reopening is a receipt for one sitting.
    #[test]
    fn a_receipt_survives_reading_the_conversation_back() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = conversation_path(dir.path(), "prj_1", "conv_1");
        append(
            &file,
            &receipt("m".into(), 1.0, "Edit".into(), "{}".into(), false),
        )
        .expect("append");
        let (messages, skipped) = devpit_agentcli::store::read(&file);
        assert_eq!(skipped, 0);
        assert!(matches!(
            &messages[0].parts[..],
            [Part::Receipt { allowed: false, .. }]
        ));
    }

    #[test]
    fn an_id_that_is_a_path_is_refused() {
        let refused = chat_receipt("../x".into(), "c".into(), "Bash".into(), "{}".into(), true)
            .expect_err("refused");
        assert_eq!(refused.code, ErrorCode::Forbidden);
        assert!(!plain("a/b") && !plain(".hidden") && !plain(""));
    }
}
