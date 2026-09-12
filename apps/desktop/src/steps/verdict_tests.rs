//! What a step's answer said about itself.
//!
//! Beside `verdict.rs` rather than in `mod_tests.rs`, where these used to sit:
//! the functions moved out of `steps/mod.rs`, and a test that stays behind is
//! a test about a file that no longer holds the thing it tests.

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
