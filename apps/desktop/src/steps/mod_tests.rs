//! The verdict and naming helpers, tested beside them.

use super::*;

const REVIEW: &str = r#"{"prompt":"review it","verdictField":"verdict","sendsBackWhen":"revise"}"#;

/// The only automatic transition in the product, and it only goes back.
#[test]
fn a_verdict_of_revise_sends_the_card_back() {
    let why = sends_back(REVIEW, r#"{"verdict":"revise","findings":["no tests"]}"#)
        .expect("should send back");
    assert!(why.contains("revise"), "{why}");
}

#[test]
fn an_approving_verdict_leaves_the_card_where_it_is() {
    assert_eq!(sends_back(REVIEW, r#"{"verdict":"approved"}"#), None);
}

/// A step that declares no verdict never moves a card on its own.
#[test]
fn a_step_without_a_verdict_never_sends_anything_back() {
    assert_eq!(
        sends_back(r#"{"prompt":"refine it"}"#, r#"{"verdict":"revise"}"#),
        None
    );
}

/// Prose where a verdict was expected is not a verdict. Reading one out of
/// it would move cards on a guess.
#[test]
fn an_answer_that_is_not_json_moves_nothing() {
    assert_eq!(sends_back(REVIEW, "I think you should revise this"), None);
}

/// A branch name has to survive a title with punctuation in it.
#[test]
fn a_card_title_becomes_a_branch_safe_name() {
    assert_eq!(slug("Fix the OAuth flow!"), "fix-the-oauth-flow");
    assert_eq!(slug("  spaces  everywhere  "), "spaces-everywhere");
    assert!(slug(&"x".repeat(80)).len() <= 40);
}

/// The same card keeps the same session id across restarts, which is what
/// makes its transcript findable later.
#[test]
fn a_card_always_gets_the_same_session_id() {
    let first = uuid_like("card_01HX");
    assert_eq!(first, uuid_like("card_01HX"));
    assert_ne!(first, uuid_like("card_01HY"));
    assert_eq!(first.len(), 36, "{first} is not shaped like a uuid");
    assert_eq!(first.matches('-').count(), 4);
}
