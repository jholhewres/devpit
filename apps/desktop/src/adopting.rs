//! Taking in a session started in a terminal, as a devpit conversation.
//!
//! Nothing is copied. The conversation's head names the CLI session, so its
//! first turn runs `--resume` into it and the CLI brings its own history; the
//! transcript devpit keeps starts empty and fills as the person speaks.

use std::path::Path;

use devpit_agentcli::head::{head_path, write_head, Head};
use devpit_agentcli::store::conversation_path;
use devpit_rpc::{ErrorCode, RpcError};

/// An id that names a file, so it may not be a path.
pub(crate) fn plain(id: &str) -> bool {
    !id.is_empty() && !id.contains(['/', '\\']) && !id.starts_with('.')
}

/// Writes the head and the empty transcript that make a conversation listed.
pub(crate) fn adopt(
    home: &Path,
    project_id: &str,
    conversation_id: &str,
    session_id: &str,
    profile_id: &str,
    title: Option<String>,
    now: f64,
) -> std::io::Result<()> {
    write_head(
        &head_path(home, project_id, conversation_id),
        &Head {
            profile: profile_id.to_owned(),
            model: None,
            card_id: None,
            created_at: now,
            cost_usd: 0.0,
            budget_usd: None,
            session_id: Some(session_id.to_owned()),
            permission: None,
            effort: None,
            title,
            rewind: Default::default(),
        },
    )?;
    let transcript = conversation_path(home, project_id, conversation_id);
    if !transcript.exists() {
        std::fs::write(transcript, b"")?;
    }
    Ok(())
}

/// `chat.adopt` — a conversation that resumes a terminal session.
///
/// Answers the new conversation's id, which the window opens as a chat tab.
#[tauri::command]
#[specta::specta]
pub fn chat_adopt(
    project_id: String,
    session_id: String,
    profile_id: String,
    title: Option<String>,
) -> Result<String, RpcError> {
    if !plain(&project_id) || !plain(&session_id) || !plain(&profile_id) {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a session"));
    }
    let conversation_id = format!("conv_{}", ulid::Ulid::generate());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default();
    adopt(
        &crate::chat::home(),
        &project_id,
        &conversation_id,
        &session_id,
        &profile_id,
        title,
        now,
    )
    .map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(conversation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_adopted_conversation_resumes_the_session_under_its_profile() {
        let home = tempfile::tempdir().expect("tempdir");
        adopt(
            home.path(),
            "prj_1",
            "conv_1",
            "aaa",
            "prof_glm",
            Some("Fix the parser".into()),
            1.0,
        )
        .expect("adopt");
        let head = devpit_agentcli::head::read_head(&head_path(home.path(), "prj_1", "conv_1"))
            .expect("head");
        assert_eq!(head.session_id.as_deref(), Some("aaa"));
        assert_eq!(head.profile, "prof_glm");
        assert_eq!(head.title.as_deref(), Some("Fix the parser"));
        // Listed like any other conversation, under the CLI's title.
        let listed = devpit_agentcli::history::conversations(home.path(), "prj_1");
        assert_eq!(listed[0].title, "Fix the parser");
    }

    #[test]
    fn ids_that_are_paths_are_refused() {
        for (project, session, profile) in [
            ("../x", "a", "p"),
            ("p", "../a", "p"),
            ("p", "a", ".hidden"),
        ] {
            let refused = chat_adopt(project.into(), session.into(), profile.into(), None)
                .expect_err("refused");
            assert_eq!(refused.code, ErrorCode::Forbidden);
        }
    }
}
