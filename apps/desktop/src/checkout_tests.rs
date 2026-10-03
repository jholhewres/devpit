use super::*;

fn step(kind: StepKind, config: &str) -> Step {
    Step {
        id: "step_1".to_owned(),
        kind,
        name: "whatever".to_owned(),
        config: config.to_owned(),
        irreversible: false,
    }
}

#[test]
fn a_session_gets_a_checkout_and_an_agent_does_not() {
    assert!(needs_worktree(StepKind::Session, "{}"));
    assert!(needs_worktree(StepKind::Command, "{}"));
    assert!(!needs_worktree(StepKind::Agent, "{}"));
}

#[test]
fn a_step_may_say_otherwise() {
    assert!(needs_worktree(
        StepKind::Agent,
        r#"{"needsWorktree": true}"#
    ));
    assert!(!needs_worktree(
        StepKind::Session,
        r#"{"needsWorktree": false}"#
    ));
}

#[test]
fn a_config_that_does_not_parse_still_gets_the_default_for_its_kind() {
    assert!(needs_worktree(StepKind::Session, "not json"));
    assert!(!needs_worktree(StepKind::Agent, "not json"));
}

/// The rule takes the step and its kind, and nothing about the column. This
/// pins that: two identical steps in columns named anything at all answer the
/// same, because the name never reaches the function.
#[test]
fn renaming_the_column_changes_nothing() {
    let doing = step(StepKind::Agent, "{}");
    let renamed = step(StepKind::Agent, "{}");
    assert_eq!(
        needs_worktree(doing.kind, &doing.config),
        needs_worktree(renamed.kind, &renamed.config)
    );
}

/// The shared variables are for a card's checkout, not the project's folder:
/// `/w/app` holds the checkouts' parent, and only what is under one counts.
#[test]
fn only_a_card_checkout_is_given_the_shared_variables() {
    let checkouts = vec!["/w/worktrees/prj/card_1".to_owned()];
    assert!(in_a_checkout(
        &checkouts,
        std::path::Path::new("/w/worktrees/prj/card_1")
    ));
    assert!(in_a_checkout(
        &checkouts,
        std::path::Path::new("/w/worktrees/prj/card_1/src")
    ));
    assert!(!in_a_checkout(
        &checkouts,
        std::path::Path::new("/w/worktrees/prj/card_10")
    ));
    assert!(!in_a_checkout(&checkouts, std::path::Path::new("/w/app")));
}
