//! The session step: a detached session for the card, attached on request.

use std::path::{Path, PathBuf};

use quockpit_agentcli as agent;
use quockpit_core::Store;
use quockpit_rpc::Step;
use serde::Deserialize;

use super::{slug, uuid_like, Finished};

/// What a `session` step needs to know, out of `step.config`.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SessionConfig {
    /// Give the session its own git worktree, named after the card.
    worktree: bool,
    model: Option<String>,
}

/// Starts a background session for this card and records the handle.
///
/// The session is not brought into a terminal here. It runs detached, and the
/// person attaches it to the target terminal when they want to sit in front of
/// it — which is the whole point of the target being one terminal rather than
/// a pane per card.
pub fn start(store: &Store, card_id: &str, step: &Step) -> Result<Finished, String> {
    let config: SessionConfig = serde_json::from_str(&step.config)
        .map_err(|err| format!("this step's config is not readable: {err}"))?;

    let card = store
        .card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("no such card")?;

    let project = store
        .project_of_card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("this card has no project on disk")?;
    let cwd = PathBuf::from(&project);

    // A stable id chosen here rather than discovered later: it is what names
    // the transcript, and the transcript is where a session the person drove
    // by hand reports what it spent.
    let session_id = uuid_like(card_id);
    let worktree = config.worktree.then(|| slug(&card.title));

    let short_id = agent::start_background(
        &cwd,
        Some(&session_id),
        worktree.as_deref(),
        config.model.as_deref(),
        super::hook_settings().as_deref(),
    )
    .map_err(|err| err.to_string())?;

    let transcript = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| agent::transcript_path(&home, &cwd, &session_id));

    // Where this front began, recorded now because it cannot be recovered
    // later: once the base branch moves, nothing on disk remembers.
    if let Some(name) = &worktree {
        let path = worktree_path(&cwd, name);
        let base = quockpit_git::head_of(&cwd).unwrap_or_default();
        let _ = store.set_card_front(
            card_id,
            path.to_str(),
            (!base.is_empty()).then_some(base.as_str()),
        );
    }

    store
        .link_session(
            card_id,
            &short_id,
            &session_id,
            transcript.as_ref().and_then(|p| p.to_str()),
        )
        .map_err(|err| err.to_string())?;

    Ok(Finished {
        ok: true,
        output: format!("session {short_id} is running; attach it to the terminal to drive it"),
        cost_usd: 0.0,
        duration_ms: 0,
        exit_code: None,
    })
}

/// Where `--worktree <name>` puts the checkout.
///
/// Under the project, in `.claude/worktrees/<name>` — read off a real run
/// rather than guessed. The first guess here was `../<name>`, which recorded a
/// path that does not exist and would have made every diff on a card fail with
/// a directory error, some way from the line that caused it.
fn worktree_path(project: &Path, name: &str) -> PathBuf {
    project.join(".claude").join("worktrees").join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_front_lands_under_the_project_not_beside_it() {
        assert_eq!(
            worktree_path(Path::new("/home/x/repo"), "fix-auth"),
            Path::new("/home/x/repo/.claude/worktrees/fix-auth")
        );
    }
}
