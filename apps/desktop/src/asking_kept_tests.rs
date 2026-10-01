use super::*;

const NPM: &str = r#"{"command":"npm test"}"#;

#[test]
fn an_edit_allowed_always_covers_every_edit_in_the_chat() {
    let kept = Kept::default();
    kept.keep("s1", rule_for("Edit", r#"{"file_path":"/w/a.rs"}"#));
    assert!(kept.covers("s1", "Write", r#"{"file_path":"/w/b.rs"}"#));
    assert!(!kept.covers("s1", "Bash", NPM));
}

/// `rm -rf build` allowed is not `rm -rf ~` allowed.
#[test]
fn a_command_allowed_always_covers_that_command_only() {
    let kept = Kept::default();
    kept.keep("s1", rule_for("Bash", NPM));
    assert!(kept.covers("s1", "Bash", r#"{"command":"  npm test "}"#));
    assert!(!kept.covers("s1", "Bash", r#"{"command":"npm test && rm -rf ~"}"#));
}

#[test]
fn a_rule_is_the_conversation_s_own() {
    let kept = Kept::default();
    kept.keep("s1", rule_for("Bash", NPM));
    assert!(!kept.covers("s2", "Bash", NPM));
}

#[test]
fn a_command_that_cannot_be_read_is_never_covered() {
    let kept = Kept::default();
    kept.keep("s1", rule_for("Bash", "not json"));
    assert!(!kept.covers("s1", "Bash", "also not json"));
}
