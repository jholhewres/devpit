use std::path::Path;

use crate::hook_settings::settings_json;

/// The reporting hooks throw the reply away; the one that can be answered
/// prints it, because that reply is the decision.
#[test]
fn only_the_hook_that_can_be_answered_prints_what_came_back() {
    let written = settings_json(Path::new("/tmp/endpoint"));
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
        "Stop",
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
    let written = settings_json(Path::new("/tmp/endpoint"));
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
    let settings = settings_json(Path::new("/home/x/.devpit/hook-endpoint"));
    assert!(settings.contains("--connect-timeout 0.5"), "{settings}");
    assert!(settings.contains("--max-time 1.5"), "{settings}");
    // A proxy in the environment must not be consulted for loopback.
    assert!(settings.contains("--noproxy"), "{settings}");
}

#[test]
fn the_endpoint_is_read_from_disk_on_every_invocation() {
    let settings = settings_json(Path::new("/home/x/.devpit/hook-endpoint"));
    // `cat` inside the command, not the address baked into it: a pty that
    // outlived a restart would otherwise post to a dead port forever.
    assert!(
        settings.contains("cat /home/x/.devpit/hook-endpoint"),
        "{settings}"
    );
}

#[test]
fn the_settings_are_json_the_cli_can_read() {
    let settings = settings_json(Path::new("/tmp/endpoint"));
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
