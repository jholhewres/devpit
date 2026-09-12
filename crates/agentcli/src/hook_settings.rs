//! The settings file that points the CLI's hooks at us.
//!
//! Written next to the state rather than into a temp file: a turn that
//! outlives the app still has a file to read, and a path that changed every
//! run would leave one on disk per card moved.

use std::path::Path;

/// The settings a turn is launched with, so its hooks reach us.
///
/// `curl` rather than a helper binary: it is already on the machine, and a
/// helper would be one more thing to ship, find and keep in step.
///
/// The timeouts are the point. This runs before every tool call the agent
/// makes, so an app that has gone away has to cost it a second and a half,
/// not a hang. `--noproxy` because a proxy in the environment must not be
/// consulted for a loopback address.
pub fn settings_json(endpoint_file: &Path) -> String {
    let file = endpoint_file.display();

    // Told and forgotten. The reply is discarded and the budget is short,
    // because these only report what happened and the agent is waiting.
    let tell = post(&file.to_string(), "1.5", false);

    // `PreToolUse` is the one that can be answered, so its reply is printed:
    // the app either sends back a decision or sends back nothing, and nothing
    // leaves the CLI's own permission mode in charge. The budget is long
    // because on this one the answer is a person.
    let consult = post(&file.to_string(), "125", true);

    let hooks: Vec<String> = [
        ("PreToolUse", &consult),
        ("PostToolUse", &tell),
        ("Stop", &tell),
        ("SubagentStop", &tell),
        ("Notification", &tell),
    ]
    .iter()
    .map(|(event, command)| {
        format!(
            "\"{event}\":[{{\"matcher\":\"*\",\"hooks\":[{{\"type\":\"command\",\
             \"command\":{}}}]}}]",
            serde_json::to_string(command).unwrap_or_else(|_| "\"true\"".to_owned())
        )
    })
    .collect();

    format!("{{\"hooks\":{{{}}}}}", hooks.join(","))
}

/// The shell one hook runs.
///
/// The endpoint is read from disk on every invocation, so a session that
/// outlived a restart posts to the new port instead of into a dead one. It
/// always exits 0: a hook that fails must not fail the turn it is reporting
/// on.
///
/// The pane rides in the query string when the terminal set one. It is the
/// only thing joining an agent's own reports to the pane somebody is looking
/// at — the hook runs three processes below the shell, and the environment is
/// what reaches that far. `${VAR:+?pane=$VAR}` is POSIX and expands to
/// nothing at all when the agent was not started in one of our terminals, so
/// a headless turn posts exactly the URL it always did.
fn post(endpoint_file: &str, seconds: &str, echo: bool) -> String {
    let sink = if echo { "" } else { " >/dev/null" };
    format!(
        "E=$(cat {endpoint_file} 2>/dev/null) && [ -n \"$E\" ] && \
         curl -sS -X POST --noproxy '*' --connect-timeout 0.5 --max-time {seconds} \
         -H 'content-type: application/json' --data-binary @- \
         \"$E${{{pane}:+?pane=${pane}}}\"{sink} 2>/dev/null || true",
        pane = devpit_tmux_pane_env()
    )
}

/// The variable a devpit terminal puts the pane's name in.
///
/// Named here rather than imported: this crate is the boundary around the
/// agent CLI and does not depend on the terminal one. The name is checked
/// against it by `the_pane_name_matches_what_the_terminal_sets`.
fn devpit_tmux_pane_env() -> &'static str {
    "DEVPIT_PANE"
}

#[cfg(test)]
#[path = "hook_settings_tests.rs"]
mod tests;
