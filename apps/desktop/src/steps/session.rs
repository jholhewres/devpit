//! The session step: a detached session for the card, attached on request.
use std::path::PathBuf;

use devpit_agentcli as agent;
use devpit_core::Store;
use devpit_rpc::Step;
use serde::Deserialize;

use super::Finished;

/// What a `session` step needs to know, out of `step.config`.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SessionConfig {
    model: Option<String>,
    /// Which profile starts it. The session outlives this step and is attached
    /// later, so the profile is written down with the link rather than looked
    /// up again from a step that may have changed by then.
    profile: Option<String>,
}

/// Starts a background session for this card and records the handle.
///
/// The session is not brought into a terminal here. It runs detached, and the
/// person attaches it to the target terminal when they want to sit in front of
/// it — which is the whole point of the target being one terminal rather than
/// a pane per card.
///
/// `session_id` is new for this run and chosen before it starts: it names the
/// transcript, and the transcript is where a session the person drove by hand
/// reports what it spent.
pub fn start(
    store: &Store,
    card_id: &str,
    step: &Step,
    session_id: &str,
) -> Result<Finished, String> {
    let config: SessionConfig = serde_json::from_str(&step.config)
        .map_err(|err| format!("this step's config is not readable: {err}"))?;

    // The card's own checkout, created here if this is the first step that
    // needs one. The CLI's `--worktree` is deliberately not used: it puts the
    // checkout inside the repository, where it shows up in the file tree, in
    // ripgrep, and one day in a commit.
    let cwd = crate::checkout::cwd_for(store, card_id, step, |_| {})?;

    let runner = config
        .profile
        .as_deref()
        .map(|id| crate::agent_profiles::runner_for(store, id))
        .transpose()?;
    let short_id = agent::start_background(
        &cwd,
        runner.as_ref(),
        Some(session_id),
        None,
        config.model.as_deref(),
        super::hook_settings().as_deref(),
    )
    .map_err(|err| err.to_string())?;

    let transcript = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| agent::transcript_path(&home, &cwd, session_id));

    store
        .link_session(
            card_id,
            &short_id,
            session_id,
            transcript.as_ref().and_then(|p| p.to_str()),
            cwd.to_str(),
            config.profile.as_deref(),
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
