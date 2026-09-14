//! Starting an agent again in a pane tmux lost.
//!
//! tmux keeps a pane's processes while the app is closed, so most of the time
//! there is nothing to do. After a reboot, or a server that died, the layout
//! still names the pane and the window comes back as a fresh shell. The agent
//! devpit had started there is typed in again, continuing its session when the
//! CLI can resume one.

use devpit_agentcli::{Event, Happening};
use devpit_core::store::pane_agents::PaneAgent;
use devpit_core::Store;

use crate::shell_launch::{profile_id, settled, to_start, Ready};

/// What restoring the pane later needs, from the agent's own hooks.
pub(crate) fn remember(pane: &str, happening: &Happening) {
    let Ok(store) = Store::open_default() else {
        return;
    };
    let _ = match &happening.event {
        Event::SessionStarted => store.remember_pane_session(
            pane,
            &happening.session_id,
            happening.transcript_path.as_deref(),
        ),
        // `/clear` ends a session and starts the next one in the same agent.
        Event::SessionEnded { reason } if reason.as_deref() != Some("clear") => {
            store.forget_pane_agent(pane)
        }
        _ => return,
    };
}

/// Types the agent back into a window tmux just made for this leaf.
///
/// On its own thread: a fresh shell takes a second or more to reach its
/// prompt, and the layout it belongs to is being opened while someone waits.
pub(crate) fn start_again(store: &Store, session: &str, leaf_id: &str) {
    let Ok(Some(agent)) = store.pane_agent(leaf_id) else {
        return;
    };
    let Ok(line) = resume_line(&agent) else {
        return;
    };
    let session = session.to_owned();
    let leaf = leaf_id.to_owned();
    std::thread::spawn(move || {
        if let Ready::Busy(_) = settled(&session, &leaf) {
            return;
        }
        let Ok(server) = crate::sessions::tmux_server() else {
            return;
        };
        let target = devpit_tmux::Server::target(&session, &leaf);
        if server.send_keys(&target, &line).is_ok() {
            let _ = server.name_pane(&target, &profile_id(&agent.launch));
        }
    });
}

fn resume_line(agent: &PaneAgent) -> Result<String, devpit_rpc::RpcError> {
    let start = to_start(&agent.launch)?;
    // Resuming a session that never wrote a line fails in the CLI; starting fresh does not.
    let written = agent
        .transcript_path
        .as_deref()
        .is_some_and(|path| std::fs::metadata(path).is_ok_and(|meta| meta.len() > 0));
    Ok(with_resume(
        &start,
        resume_flag_of(&agent.launch),
        agent.session_id.as_deref().filter(|_| written),
    ))
}

fn resume_flag_of(launch: &str) -> Option<&'static str> {
    let base = crate::projects::store()
        .ok()
        .and_then(|store| crate::agent_profiles::all(&store).ok())
        .and_then(|profiles| profiles.into_iter().find(|one| one.id == launch))
        .map(|profile| profile.base)
        .unwrap_or_else(|| launch.to_owned());
    devpit_pty::agents::known(&base).and_then(|agent| agent.resume_flag)
}

/// The launch line, continuing `session` when the agent can.
///
/// The id is typed into a shell, so anything but a session id's own
/// characters is refused rather than quoted.
fn with_resume(start: &str, flag: Option<&str>, session: Option<&str>) -> String {
    let plain = |id: &&str| {
        !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
    };
    match (flag, session.filter(plain)) {
        (Some(flag), Some(id)) => format!("{start} {}", flag.replace("{}", id)),
        _ => start.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::with_resume;

    const START: &str = "claude --settings /s.json";

    #[test]
    fn a_known_session_is_continued() {
        assert_eq!(
            with_resume(
                START,
                Some("--resume {}"),
                Some("94e3457c-dcfa-4580-951a-dc9110a9580e")
            ),
            "claude --settings /s.json --resume 94e3457c-dcfa-4580-951a-dc9110a9580e"
        );
    }

    #[test]
    fn without_a_session_or_a_resume_flag_the_agent_starts_fresh() {
        assert_eq!(with_resume(START, Some("--resume {}"), None), START);
        assert_eq!(with_resume(START, None, Some("94e3457c")), START);
    }

    /// The line is typed into a shell: an id that is not an id never reaches it.
    #[test]
    fn an_id_that_could_say_something_to_the_shell_is_refused() {
        for id in ["abc; rm -rf ~", "$(id)", "../x", ""] {
            assert_eq!(
                with_resume(START, Some("--resume {}"), Some(id)),
                START,
                "{id}"
            );
        }
    }
}
