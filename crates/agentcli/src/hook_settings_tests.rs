use std::path::Path;

use crate::hook_settings::{plugin_hooks_json, settings_json, HOOKED};

/// The reporting hooks throw the reply away; the one that can be answered
/// prints it, because that reply is the decision.
#[test]
fn only_the_hook_that_can_be_answered_prints_what_came_back() {
    let written = settings_json(Path::new("/tmp/endpoint"), Path::new("/tmp/hook-auth"));
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("valid json");

    let command = |event: &str| -> String {
        parsed["hooks"][event][0]["hooks"][0]["command"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };

    // The stdout sink, not the stderr one: both hooks silence stderr, and
    // only one of them silences the answer.
    assert!(
        !command("PreToolUse").contains(" >/dev/null"),
        "the decision was thrown away"
    );
    for reporting in [
        "PostToolUse",
        "UserPromptSubmit",
        "Stop",
        "StopFailure",
        "SubagentStart",
        "SubagentStop",
        "Notification",
        "SessionStart",
        "SessionEnd",
    ] {
        assert!(
            command(reporting).contains(" >/dev/null"),
            "{reporting} prints a reply nobody reads back into the turn"
        );
    }
}

/// A person is the answer on `PreToolUse`, so it waits; the others report and
/// must not hold the turn.
#[test]
fn the_hook_that_waits_on_a_person_gets_a_longer_budget() {
    let written = settings_json(Path::new("/tmp/endpoint"), Path::new("/tmp/hook-auth"));
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("valid json");
    let command = |event: &str| -> String {
        parsed["hooks"][event][0]["hooks"][0]["command"]
            .as_str()
            .unwrap_or_default()
            .to_owned()
    };

    assert!(command("PreToolUse").contains("--max-time 125"));
    assert!(command("Stop").contains("--max-time 1.5"));
}

/// The two rules the settings exist to keep.
#[test]
fn the_hook_command_gives_up_rather_than_holding_the_agent() {
    let settings = settings_json(
        Path::new("/home/x/.devpit/hook-endpoint"),
        Path::new("/home/x/.devpit/hook-auth"),
    );
    assert!(settings.contains("--connect-timeout 0.5"), "{settings}");
    assert!(settings.contains("--max-time 1.5"), "{settings}");
    // A proxy in the environment must not be consulted for loopback.
    assert!(settings.contains("--noproxy"), "{settings}");
}

#[test]
fn the_endpoint_is_read_from_disk_on_every_invocation() {
    let settings = settings_json(
        Path::new("/home/x/.devpit/hook-endpoint"),
        Path::new("/home/x/.devpit/hook-auth"),
    );
    // `cat` inside the command, not the address baked into it: a pty that
    // outlived a restart would otherwise post to a dead port forever.
    assert!(
        settings.contains("cat /home/x/.devpit/hook-endpoint"),
        "{settings}"
    );
}

#[test]
fn the_settings_are_json_the_cli_can_read() {
    let settings = settings_json(Path::new("/tmp/endpoint"), Path::new("/tmp/hook-auth"));
    let parsed: serde_json::Value = serde_json::from_str(&settings).expect("valid JSON");
    let hooks = parsed
        .get("hooks")
        .expect("hooks")
        .as_object()
        .expect("object");
    for event in ["PreToolUse", "PostToolUse", "Stop", "Notification"] {
        assert!(hooks.contains_key(event), "no {event} in {settings}");
    }
}

/// The secret rides in a header read from a file, never in the command itself.
///
/// Everything in that command line is visible to anything on the machine that
/// can list processes, and the settings file it lives in is read by the agent
/// CLI — so the value stays on disk, owner-only, and `curl` picks it up there.
#[test]
fn the_secret_is_read_from_a_file_and_never_typed_into_the_command() {
    let settings = settings_json(
        Path::new("/home/x/.devpit/hook-endpoint"),
        Path::new("/home/x/.devpit/hook-auth"),
    );

    assert!(
        settings.contains("-H @'/home/x/.devpit/hook-auth'"),
        "{settings}"
    );
    assert!(
        !settings.contains("x-devpit-hook:"),
        "the secret's header was written into the command: {settings}"
    );
}

/// devpit's read tools are allowed without a question; its writes still ask.
#[test]
fn the_board_can_be_read_without_asking_and_written_only_by_asking() {
    let written = settings_json(Path::new("/tmp/endpoint"), Path::new("/tmp/hook-auth"));
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("valid json");
    let allowed: Vec<&str> = parsed["permissions"]["allow"]
        .as_array()
        .expect("allow list")
        .iter()
        .filter_map(|one| one.as_str())
        .collect();
    assert!(allowed.contains(&"mcp__devpit__devpit_board"));
    assert!(allowed
        .iter()
        .all(|tool| !tool.contains("comment") && !tool.contains("move")));
}

/// A session launched with the settings says so to its hooks, which is what
/// keeps the plugin's copies from reporting the same event again.
#[test]
fn the_settings_mark_the_session_as_already_reporting() {
    let written = settings_json(Path::new("/tmp/endpoint"), Path::new("/tmp/hook-auth"));
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("valid json");
    assert_eq!(parsed["env"][HOOKED], "1");
}

/// The plugin's hooks run only inside a devpit terminal, and only where the
/// launch line did not already bring the same hooks.
#[test]
fn the_plugin_hooks_stay_quiet_outside_a_pane_and_beside_the_settings() {
    let written = plugin_hooks_json(Path::new("/tmp/endpoint"), Path::new("/tmp/hook-auth"));
    let parsed: serde_json::Value = serde_json::from_str(&written).expect("valid json");
    let hooks = parsed["hooks"].as_object().expect("hooks");
    assert!(hooks.contains_key("PreToolUse") && hooks.contains_key("SessionStart"));
    for (event, entries) in hooks {
        let command = entries[0]["hooks"][0]["command"]
            .as_str()
            .unwrap_or_default();
        assert!(
            command.starts_with("[ -n \"$DEVPIT_PANE\" ] && [ -z \"$DEVPIT_HOOKED\" ] || exit 0; "),
            "{event}: {command}"
        );
    }

    let run = |env: &[(&str, &str)]| {
        let command = hooks["Stop"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap_or_default();
        // The post replaced by a word, so what is tested is the guard.
        let guard = command.split("; ").next().unwrap_or_default();
        let mut shell = std::process::Command::new("sh");
        shell.arg("-c").arg(format!("{guard}; echo posted"));
        shell.env_remove("DEVPIT_PANE").env_remove(HOOKED);
        for (name, value) in env {
            shell.env(name, value);
        }
        String::from_utf8_lossy(&shell.output().expect("sh").stdout)
            .trim()
            .to_owned()
    };
    assert_eq!(run(&[]), "");
    assert_eq!(run(&[("DEVPIT_PANE", "leaf_1")]), "posted");
    assert_eq!(run(&[("DEVPIT_PANE", "leaf_1"), (HOOKED, "1")]), "");
}
