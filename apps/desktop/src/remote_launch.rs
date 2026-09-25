//! An orchestrator's conversation, continued where a phone can reach it.
//!
//! Remote Control belongs to an interactive session: a chat's process takes
//! the flag and does nothing with it (measured on Claude Code 2.1.282). So
//! going remote is the same conversation resumed in a terminal with
//! `--remote-control`, and the chat's own process let go first — two
//! processes on one session would answer each other's messages.

use devpit_agentcli::head::{head_path, read_head};
use devpit_rpc::{ErrorCode, RpcError};

/// The line that resumes `conversation` in a terminal, reachable remotely.
pub(crate) fn remote_line(
    app: &tauri::AppHandle,
    project_id: &str,
    conversation: &str,
) -> Result<String, RpcError> {
    let home = crate::projects::project_home(project_id)?;
    let head = read_head(&head_path(&home.sessions(), conversation)).ok_or_else(|| {
        RpcError::new(ErrorCode::NotFound, "this conversation has not started yet")
    })?;
    let session = head
        .session_id
        .as_deref()
        .filter(|id| plain_session(id))
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                "this conversation has no session to resume yet",
            )
        })?;
    let store = crate::projects::store()?;
    let name = store
        .projects()?
        .into_iter()
        .find(|row| row.id == project_id)
        .map(|row| row.name)
        .unwrap_or_default();
    crate::chat_resident::release(app, conversation);
    let start = crate::shell_launch::to_start(&head.profile)?;
    Ok(format!(
        "{start} --resume {session} --remote-control {}",
        remote_name(&name)
    ))
}

/// A session id as the CLI writes one. Anything else is not typed.
pub(crate) fn plain_session(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
}

/// The name the session shows in claude.ai and the app: the orchestrator's
/// name made plain, so nothing a person typed is ever shell syntax here.
pub(crate) fn remote_name(name: &str) -> String {
    let mut out = String::from("devpit");
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            if out.ends_with("devpit") && out.len() == 6 {
                out.push('-');
            }
            out.push(ch);
        } else if !out.ends_with('-') && out.len() > 6 {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(48).collect()
}

#[cfg(test)]
mod tests {
    use super::{plain_session, remote_name};

    #[test]
    fn only_a_plain_session_id_and_a_plain_name_are_typed() {
        assert!(plain_session("9c684740-743b-4394-8b4c-c11dd543c135"));
        assert!(!plain_session("abc; rm -rf /"));
        assert!(!plain_session(""));
        assert_eq!(remote_name("Client work"), "devpit-client-work");
        assert_eq!(remote_name("x'; reboot #"), "devpit-x-reboot");
        assert_eq!(remote_name("   "), "devpit");
    }
}
