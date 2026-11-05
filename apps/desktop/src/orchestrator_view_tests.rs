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
    assert!(hooked(Some("not json")).is_none());
    assert!(hooked(Some("[]")).is_none());
}
