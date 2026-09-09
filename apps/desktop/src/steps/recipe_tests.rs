use super::*;

fn agents() -> Vec<String> {
    vec!["architect".to_owned()]
}

fn skills() -> Vec<String> {
    vec!["tdd".to_owned()]
}

fn check(config: &str) -> Option<String> {
    refuse(StepKind::Agent, config, &agents(), &skills())
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
    assert_eq!(refuse(StepKind::Command, "{}", &agents(), &skills()), None);
    assert_eq!(refuse(StepKind::Session, "{}", &agents(), &skills()), None);
}
