//! What the window over an orchestrator's chat asks about one session: where
//! its folder stands, and a new name for it.
//!
//! Both are the person's, from the window. No agent reaches them: renaming a
//! session changes what every other session messages it by.

use std::path::Path;

use devpit_rpc::{ErrorCode, RpcError, SessionChanges};

/// The most of a diff the window reads. A diff past it is cut, and says so.
const MOST_DIFF: usize = 512 * 1024;

/// The most commits listed: the window is a glance, the project has history.
const MOST_COMMITS: u32 = 20;

/// `session.changes` — a session's folder, as git sees it.
///
/// Only a folder inside one of devpit's projects or their checkouts: the
/// window shows sessions devpit placed, and a path from anywhere else is not
/// one of them.
#[tauri::command]
#[specta::specta]
pub async fn session_changes(
    cwd: String,
    card_id: Option<String>,
) -> Result<SessionChanges, RpcError> {
    crate::off_main::blocking(move || session_changes_now(&cwd, card_id.as_deref())).await
}

fn session_changes_now(cwd: &str, card_id: Option<&str>) -> Result<SessionChanges, RpcError> {
    let folder = Path::new(cwd);
    let projects = crate::live_sessions::recent_projects()?;
    let project = crate::agent_api::project_at(&projects, folder).ok_or_else(|| {
        RpcError::new(
            ErrorCode::NotFound,
            "that folder is not in a devpit project",
        )
    })?;
    let failed = |err: devpit_git::GitError| RpcError::internal(err.to_string());
    let status = devpit_git::status(folder).map_err(failed)?;
    // A card's work is counted from where the card began, not from whatever
    // its upstream has become.
    let card_base = card_id
        .and_then(|id| crate::board::store().ok()?.card(id).ok().flatten())
        .filter(|card| {
            card.worktree_path
                .as_deref()
                .is_some_and(|path| same(path, cwd))
        })
        .and_then(|card| card.base_ref);
    let base = card_base
        .clone()
        .or_else(|| devpit_git::upstream_of(folder));
    let commits = base
        .as_deref()
        .map(|base| devpit_git::commits_in(folder, &format!("{base}..HEAD"), MOST_COMMITS))
        .unwrap_or_default();
    let mut diff =
        devpit_git::diff_since(folder, card_base.as_deref().unwrap_or("HEAD")).unwrap_or_default();
    let cut = diff.len() > MOST_DIFF;
    if cut {
        let mut at = MOST_DIFF;
        while !diff.is_char_boundary(at) {
            at -= 1;
        }
        diff.truncate(at);
    }
    Ok(SessionChanges {
        branch: status.branch,
        folder: folder
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| cwd.to_owned()),
        worktree: !same(&project.root_path, cwd),
        ahead: status.ahead,
        behind: status.behind,
        base: card_base
            .map(|base| base.chars().take(12).collect())
            .or(base),
        changes: devpit_git::changes(folder).map_err(failed)?,
        commits,
        diff,
        cut,
    })
}

fn same(one: &str, other: &str) -> bool {
    let resolved = |path: &str| std::fs::canonicalize(path).unwrap_or_else(|_| path.into());
    resolved(one) == resolved(other)
}

/// Whether `name` is one a session can be called by: a word others type,
/// with no space, slash or line in it.
pub(crate) fn nameable(name: &str) -> bool {
    !name.is_empty()
        && name.chars().count() <= 64
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// `orchestrator.rename` — calls a session something else, by typing the
/// CLI's own `/rename` into its terminal: the name lives in the CLI, and one
/// written anywhere else would be written over.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_rename(
    profile_id: String,
    name: String,
    to: String,
) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let to = to.trim();
        if !nameable(to) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "a name is up to 64 letters, digits, dots, dashes or underscores",
            ));
        }
        if to == name {
            return Ok(());
        }
        if crate::live_sessions::running_named(&profile_id, to).is_ok_and(|found| !found.is_empty())
        {
            return Err(RpcError::new(
                ErrorCode::Conflict,
                format!("a session is already called {to}"),
            ));
        }
        let target = crate::live_sessions::terminal_of(&profile_id, &name)?;
        let tmux = crate::sessions::tmux_server()?;
        if tmux
            .shell_in_front(&target)
            .map_err(|err| RpcError::internal(err.to_string()))?
        {
            return Err(RpcError::new(
                ErrorCode::Conflict,
                "the agent is not in front in that terminal — open it to see why",
            ));
        }
        // Typed into a question, the keys would answer it.
        let asking = crate::live_sessions::screen_of(&target)
            .and_then(|shown| crate::live_prompt::pending(&shown))
            .is_some();
        if asking {
            return Err(RpcError::new(
                ErrorCode::Conflict,
                "it is asking you something — answer it first",
            ));
        }
        tmux.paste_and_send(&target, &format!("/rename {to}"))
            .map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::nameable;

    #[test]
    fn a_name_is_a_word_others_can_type() {
        assert!(nameable("devpit-02"));
        assert!(nameable("invoice_fix.v2"));
        assert!(nameable("revisão"));
        assert!(!nameable(""));
        assert!(!nameable("two words"));
        assert!(!nameable("a/b"));
        assert!(!nameable("x\n/clear"));
        assert!(!nameable(&"a".repeat(65)));
    }
}
