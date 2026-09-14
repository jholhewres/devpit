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

fn check(config: &str) -> Option<String> {
    refuse(StepKind::Agent, config, &agents(), &skills(), &profiles())
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
fn the_other_kinds_are_left_alone() {
    assert_eq!(
        refuse(StepKind::Command, "{}", &agents(), &skills(), &profiles()),
        None
    );
    assert_eq!(
        refuse(StepKind::Session, "{}", &agents(), &skills(), &profiles()),
        None
    );
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
