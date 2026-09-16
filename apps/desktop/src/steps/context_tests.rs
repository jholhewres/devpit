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
        base_ref: Some("9f1c2b7".to_owned()),
        project: Some("devpit".to_owned()),
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

/// A card, as the store hands one over.
fn card() -> devpit_core::CardRow {
    devpit_core::CardRow {
        id: "card_1".to_owned(),
        column_id: "col_1".to_owned(),
        title: "Fix it".to_owned(),
        body: "the body".to_owned(),
        position: 0,
        worktree_path: Some("/home/me/.devpit/worktrees/p/card_1".to_owned()),
        base_ref: Some("9f1c2b7e4a".to_owned()),
        due_at: None,
    }
}

/// The bug this settles: `DEVPIT_BRANCH` was `card.base_ref`, which is the
/// commit the card started from. A step doing `git push origin $DEVPIT_BRANCH`
/// was pushing a SHA.
#[test]
fn the_branch_variable_is_the_branch_not_the_sha() {
    let cwd = std::path::Path::new("/home/me/.devpit/worktrees/p/card_1");
    let context = context_of(
        &card(),
        Some(Checkout {
            path: cwd,
            branch: "devpit/fix-it-card1",
        }),
        Some("/home/me/work/devpit"),
    );

    assert_eq!(context.branch.as_deref(), Some("devpit/fix-it-card1"));
    assert_eq!(context.base_ref.as_deref(), Some("9f1c2b7e4a"));

    let vars = injected(&context, &["branch".to_owned(), "baseRef".to_owned()]);
    assert_eq!(
        vars,
        vec![
            ("DEVPIT_BRANCH".to_owned(), "devpit/fix-it-card1".to_owned()),
            ("DEVPIT_BASE_REF".to_owned(), "9f1c2b7e4a".to_owned()),
        ]
    );
}

/// A step that asked for no checkout runs in the project, on whatever branch
/// the person left it on — which is not this card's, and saying it is would be
/// a claim about somebody else's work.
#[test]
fn without_a_checkout_there_is_no_branch_to_name() {
    let context = context_of(&card(), None, Some("/home/me/work/devpit"));
    assert_eq!(context.branch, None);
    assert!(injected(&context, &["branch".to_owned()]).is_empty());
    // The base commit is the card's either way.
    assert_eq!(context.base_ref.as_deref(), Some("9f1c2b7e4a"));
}

/// The command runner reads the same facts through a context of its own, where
/// everything absent is empty: a `{{key}}` always expands to something.
#[test]
fn a_command_step_reads_the_same_branch_and_base() {
    let context = context_of(&card(), None, Some("/home/me/work/devpit")).for_a_command();
    assert_eq!(context.branch, "");
    assert_eq!(context.base_ref, "9f1c2b7e4a");
    assert_eq!(context.project, "devpit");
    assert_eq!(context.card_title, "Fix it");
}
