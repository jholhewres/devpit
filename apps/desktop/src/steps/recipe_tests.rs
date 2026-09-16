use super::*;

/// The profiles a machine might have. A test that names one has to say so.
fn profiles() -> Vec<String> {
    vec!["01JGLM".to_owned()]
}

fn agents() -> Vec<String> {
    vec!["architect".to_owned()]
}

fn skills() -> Vec<String> {
    vec!["tdd".to_owned()]
}

fn check_kind(kind: StepKind, config: &str) -> Option<String> {
    refuse(kind, config, &agents(), &skills(), &profiles())
}

fn check(config: &str) -> Option<String> {
    check_kind(StepKind::Agent, config)
}

#[test]
fn a_step_with_no_cap_is_refused_and_says_so() {
    let why = check(r#"{"prompt": "go"}"#).expect("refused");
    assert!(
        why.contains("cap"),
        "the refusal does not mention the cap: {why}"
    );
}

#[test]
fn an_agent_that_does_not_exist_is_refused_by_name() {
    let why = check(r#"{"prompt": "go", "capUsd": 1, "agent": "architekt"}"#).expect("refused");
    assert!(
        why.contains("architekt"),
        "the refusal does not name it: {why}"
    );
}

#[test]
fn a_skill_that_does_not_exist_is_refused_by_name() {
    let why = check(r#"{"prompt": "go", "capUsd": 1, "skills": ["tddd"]}"#).expect("refused");
    assert!(why.contains("tddd"), "the refusal does not name it: {why}");
}

#[test]
fn a_context_key_that_does_not_exist_is_refused_and_the_real_ones_are_listed() {
    let why = check(r#"{"prompt": "go", "capUsd": 1, "inject": ["brunch"]}"#).expect("refused");
    assert!(why.contains("brunch"));
    assert!(
        why.contains("branch"),
        "the refusal does not offer the real keys: {why}"
    );
}

#[test]
fn a_recipe_that_names_only_what_exists_is_saved() {
    assert_eq!(
        check(
            r#"{"prompt": "go", "capUsd": 2, "agent": "architect",
                "skills": ["tdd"], "inject": ["branch", "cardTitle"]}"#
        ),
        None
    );
}

/// Only an agent step has a recipe. A command step declares a command, and a
/// session declares a model; neither has a cap to miss.
#[test]
fn the_other_kinds_are_not_asked_for_a_cap() {
    assert_eq!(
        check_kind(StepKind::Command, r#"{"command":"make test"}"#),
        None
    );
    assert_eq!(check_kind(StepKind::Session, "{}"), None);
}

/// What the form used to save: the single line the person typed. It parsed
/// into nothing, was accepted, and failed when a card landed on the lane.
#[test]
fn a_config_that_is_not_json_is_refused_for_every_kind() {
    for kind in [StepKind::Agent, StepKind::Command, StepKind::Session] {
        let why = check_kind(kind, "make test").expect("refused");
        assert!(why.contains("not readable"), "{kind:?}: {why}");
    }
}

#[test]
fn a_command_step_with_no_command_is_refused() {
    assert!(check_kind(StepKind::Command, "{}").is_some());
}

/// The configs the form makes, read from the fixture the web test reads too.
///
/// Shared rather than copied: this is one rule with a half on each side of the
/// bridge, and two fixtures would agree for about a week.
#[test]
fn a_step_made_in_the_form_runs() {
    #[derive(serde::Deserialize)]
    struct Case {
        kind: String,
        config: serde_json::Value,
    }
    #[derive(serde::Deserialize)]
    struct Cases {
        cases: Vec<Case>,
    }

    let text = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/steps/tests/fixtures/step-configs.json"
    ))
    .expect("the shared fixture");
    let cases: Cases = serde_json::from_str(&text).expect("the fixture is readable");
    assert!(!cases.cases.is_empty(), "the fixture has no cases");

    for case in cases.cases {
        let config = case.config.to_string();
        match case.kind.as_str() {
            "command" => {
                assert_eq!(check_kind(StepKind::Command, &config), None, "{config}");
                devpit_steps::validate(&config).expect("the runner reads it");
            }
            "session" => {
                assert_eq!(check_kind(StepKind::Session, &config), None, "{config}");
                crate::steps::session::readable(&config).expect("the runner reads it");
            }
            "agent" => {
                assert_eq!(check_kind(StepKind::Agent, &config), None, "{config}");
                crate::steps::agent_config::readable(&config).expect("the runner reads it");
            }
            other => panic!("the fixture has a kind nothing runs: {other}"),
        }
    }
}

#[test]
fn a_step_naming_a_profile_that_exists_is_saved() {
    assert_eq!(
        check(r#"{"capUsd":1.0,"profile":"01JGLM"}"#),
        None,
        "a real profile was refused"
    );
}

#[test]
fn a_step_naming_a_profile_that_does_not_exist_is_refused() {
    // At save time, while the person is still looking at what they typed —
    // not when a card lands on the lane two screens away.
    assert!(check(r#"{"capUsd":1.0,"profile":"01JGONE"}"#).is_some());
}

#[test]
fn a_subagent_is_not_checked_against_the_profiles() {
    // They share the JSON and nothing else. `architect` is a frontmatter name
    // and not a profile id, and naming it must not be read as naming one.
    assert_eq!(check(r#"{"capUsd":1.0,"agent":"architect"}"#), None);
}
