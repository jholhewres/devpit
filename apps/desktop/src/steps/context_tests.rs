use super::*;

fn context() -> Context {
    Context {
        card: "card_1".to_owned(),
        card_title: "Fix it".to_owned(),
        // A branch name is not a safe string, and this is the one that proves
        // the difference between a value and a command.
        branch: Some("fix; rm -rf /".to_owned()),
        ..Context::default()
    }
}

#[test]
fn a_branch_with_a_semicolon_in_it_arrives_as_a_value() {
    let vars = injected(&context(), &["branch".to_owned()]);
    assert_eq!(
        vars,
        vec![("DEVPIT_BRANCH".to_owned(), "fix; rm -rf /".to_owned())],
        "the branch was not passed as one variable's value"
    );
}

#[test]
fn a_key_is_shouted_the_way_environment_variables_are() {
    let vars = injected(&context(), &["cardTitle".to_owned()]);
    assert_eq!(vars[0].0, "DEVPIT_CARD_TITLE");
}

#[test]
fn a_key_nothing_answers_is_left_out_rather_than_sent_empty() {
    let vars = injected(&context(), &["worktreePath".to_owned()]);
    assert!(
        vars.is_empty(),
        "an unknown value was sent as an empty variable"
    );
}

#[test]
fn a_key_that_is_not_a_context_key_is_ignored() {
    assert!(injected(&context(), &["brunch".to_owned()]).is_empty());
}

#[test]
fn every_key_the_validator_offers_can_actually_be_injected() {
    let full = Context {
        branch: Some("main".to_owned()),
        worktree_path: Some("/tmp/wt".to_owned()),
        project_path: Some("/tmp/p".to_owned()),
        ..context()
    };
    let keys: Vec<String> = CONTEXT_KEYS.iter().map(|key| (*key).to_owned()).collect();
    assert_eq!(
        injected(&full, &keys).len(),
        CONTEXT_KEYS.len(),
        "a key the validator accepts injects nothing"
    );
}
