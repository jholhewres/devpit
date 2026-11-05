use super::*;

use crate::orchestrator_notes::{note, Section};

const PREFERENCES: &str = "# Preferences

## Rules for sessions

- Commit messages in English.

## Orchestrator (not for sessions)

- Never end a session on your own: only when the person asks.

## Rules for sessions: billing

- Deploy after the gate.

## Log

- 2026-11-01 — likes short reports
";

#[test]
fn the_orchestrator_sees_its_own_preferences_and_not_the_rules_for_sessions() {
    let shown = in_view(PREFERENCES);
    assert!(shown.contains("Never end a session on your own"), "{shown}");
    assert!(shown.contains("likes short reports"), "{shown}");
    assert!(!shown.contains("Commit messages in English"), "{shown}");
    assert!(!shown.contains("Deploy after the gate"), "{shown}");
}

#[test]
fn a_long_log_is_cut_in_the_middle_keeping_the_standing_ones_and_the_newest() {
    let old: String = (0..400)
        .map(|n| format!("- 2026-01-01 — old entry number {n}\n"))
        .collect();
    let text = format!("{PREFERENCES}{old}- 2026-11-05 — the newest one\n");
    let shown = in_view(&text);
    assert!(shown.chars().count() < HEAD + TAIL + 200, "{}", shown.len());
    assert!(shown.contains("Never end a session on your own"));
    assert!(shown.contains("the newest one"));
    assert!(shown.contains("read them in context/preferences.md"));
}

#[test]
fn an_orchestrator_folder_hands_its_preferences_to_every_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path();
    std::fs::create_dir_all(folder.join("context")).expect("mkdir");
    std::fs::write(folder.join("context/preferences.md"), PREFERENCES).expect("prefs");
    std::fs::create_dir_all(folder.join(".claude")).expect("mkdir");
    std::fs::write(
        settings_file(folder),
        r#"{"permissions":{"allow":["Bash(ls:*)"]}}"#,
    )
    .expect("theirs");

    set_up(folder).expect("set up");
    set_up(folder).expect("again");

    let view = std::fs::read_to_string(folder.join(VIEW)).expect("view");
    assert!(view.contains("Never end a session on your own"), "{view}");
    let settings: Value =
        serde_json::from_str(&std::fs::read_to_string(settings_file(folder)).expect("settings"))
            .expect("json");
    assert_eq!(settings["permissions"]["allow"][0], "Bash(ls:*)");
    let on_prompt = settings["hooks"]["UserPromptSubmit"]
        .as_array()
        .expect("hook");
    assert_eq!(on_prompt.len(), 1, "set once: {on_prompt:?}");
    assert!(on_prompt[0].to_string().contains(VIEW));
}

#[test]
fn a_preference_logged_is_in_view_on_the_next_turn() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path();
    std::fs::create_dir_all(folder.join("context")).expect("mkdir");
    std::fs::write(folder.join("context/preferences.md"), PREFERENCES).expect("prefs");
    set_up(folder).expect("set up");

    note(
        folder,
        "preferences",
        "no drafts unless asked",
        Section::Log,
    )
    .expect("note");

    let view = std::fs::read_to_string(folder.join(VIEW)).expect("view");
    assert!(view.contains("no drafts unless asked"), "{view}");
}

#[test]
fn settings_that_are_not_an_object_are_left_alone() {
    assert!(hooked(Some("not json"), COMMAND).is_none());
    assert!(hooked(Some("[]"), COMMAND).is_none());
}

/// The hook rebuilds the copy before printing it: a hand edit of
/// preferences.md, by the person or the orchestrator, is in view at the very
/// next message, with no opening in between.
#[test]
fn a_hand_edit_of_the_preferences_is_in_view_at_the_next_message() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("orchestrator").join("me");
    std::fs::create_dir_all(folder.join("context")).expect("mkdir");
    std::fs::write(folder.join("context/preferences.md"), PREFERENCES).expect("prefs");
    set_up(&folder).expect("set up");

    let edited = PREFERENCES.replace("likes short reports", "likes long reports");
    std::fs::write(folder.join("context/preferences.md"), edited).expect("hand edit");

    let shown = shown_now(&folder, &dir.path().join("orchestrator"));
    assert!(shown.contains("likes long reports"), "{shown}");
    assert!(!shown.contains("likes short reports"), "{shown}");
    let elsewhere = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        shown_now(&folder, elsewhere.path()),
        "",
        "only under devpit's own folder"
    );
}

/// devpit itself prints the copy, and the copy as it stands is the fallback
/// when that fails; an older hook is replaced, not doubled.
#[test]
fn the_hook_asks_devpit_and_replaces_an_older_one() {
    let line = hook_command(Some(Path::new("/opt/devpit app/devpit")));
    assert!(
        line.starts_with("'/opt/devpit app/devpit' preferences-in-view \"$CLAUDE_PROJECT_DIR\""),
        "{line}"
    );
    assert!(line.ends_with(COMMAND), "{line}");
    let older = json!({ "hooks": { "UserPromptSubmit": [
        { "hooks": [{ "type": "command", "command": COMMAND }] },
        { "hooks": [{ "type": "command", "command": "echo theirs" }] }
    ] } })
    .to_string();
    let settings = hooked(Some(&older), &line).expect("settings");
    let on_prompt = settings["hooks"]["UserPromptSubmit"]
        .as_array()
        .expect("hooks");
    assert_eq!(on_prompt.len(), 2, "{on_prompt:?}");
    assert_eq!(on_prompt[0]["hooks"][0]["command"], line.as_str());
    assert_eq!(on_prompt[1]["hooks"][0]["command"], "echo theirs");
}
