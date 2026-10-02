use super::*;

#[test]
fn a_log_is_only_one_the_brief_names() {
    let folder = Path::new("/o");
    assert_eq!(
        log_file(folder, "sessions"),
        Some(PathBuf::from("/o/context/sessions.md"))
    );
    assert_eq!(log_file(folder, "../../.bashrc"), None);
    assert_eq!(log_file(folder, "notes/api"), None);
}

#[test]
fn an_entry_is_a_dated_bullet_with_its_lines_under_it() {
    assert_eq!(
        entry("2026-10-01", "orch-55319: handed the fix\nPR #12 open").as_deref(),
        Ok("- 2026-10-01 — orch-55319: handed the fix\n  PR #12 open\n")
    );
    assert!(entry("2026-10-01", "   ").is_err());
    assert!(entry("2026-10-01", "bad\u{1b}[2J").is_err());
}

#[test]
fn a_note_is_appended_and_what_was_there_stays() {
    let dir = tempfile::tempdir().expect("tempdir");
    note(dir.path(), "sessions", "first").expect("first");
    note(dir.path(), "sessions", "second").expect("second");
    let written = std::fs::read_to_string(dir.path().join("context/sessions.md")).expect("read");
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].ends_with("— first") && lines[1].ends_with("— second"));
}

#[test]
fn the_rules_for_sessions_go_with_every_brief() {
    let text = "# Preferences\n\n- likes small commits\n\n## Rules for sessions\n\n- Never push to main.\n- Write commit messages in English.\n\n## Other\n- not this\n";
    assert_eq!(
        super::rules_in(text).as_deref(),
        Some("- Never push to main.\n- Write commit messages in English.")
    );
    assert_eq!(super::rules_in("# Preferences\n- nothing here\n"), None);
    assert_eq!(super::rules_in("## Rules for sessions\n\n"), None);

    let dir = tempfile::tempdir().expect("dir");
    std::fs::create_dir_all(dir.path().join("context")).expect("context");
    std::fs::write(dir.path().join("context/preferences.md"), text).expect("prefs");
    let brief = super::briefed(dir.path(), "Fix the invoice total.");
    assert_eq!(
        brief,
        "Fix the invoice total.\n\n## Rules for sessions\n\n- Never push to main.\n- Write commit messages in English."
    );
    // Too long already, it goes as it is rather than being refused.
    let long = "x".repeat(7990);
    assert_eq!(super::briefed(dir.path(), &long), long);
}
