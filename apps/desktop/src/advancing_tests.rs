//! The rule that moves a card nobody is watching.
//!
//! Every case here is a way somebody's work could be moved for a reason they
//! did not choose, so each one is stated rather than left to the reader of the
//! `match`.

use super::*;
use devpit_rpc::StepKind;

fn lane(autonomy: Autonomy, on_pass: Option<&str>) -> Lane<'_> {
    Lane {
        autonomy,
        on_pass,
        target_is_irreversible: false,
    }
}

fn step(config: &str) -> Step {
    Step {
        id: "step_1".to_owned(),
        kind: StepKind::Agent,
        name: "review".to_owned(),
        config: config.to_owned(),
        irreversible: false,
    }
}

const DECLARED: &str = r#"{"verdictField":"verdict","sendsBackWhen":"revise"}"#;

// ── Backward: the direction that is never a surprise ──────────────────

#[test]
fn a_refusal_sends_the_card_back_even_from_a_manual_lane() {
    // The agent saying its own answer was not good enough. That is not the
    // product taking initiative, so autonomy does not gate it.
    let back = decide(
        &lane(Autonomy::Manual, None),
        &Verdict::SendBack("verdict: revise".to_owned()),
        Some("col_before"),
    );
    assert_eq!(
        back,
        Move::Back {
            to: "col_before".to_owned(),
            why: "verdict: revise".to_owned()
        }
    );
}

#[test]
fn a_refusal_with_nowhere_to_go_leaves_the_card_alone() {
    // A card created in this lane has no previous column, and inventing one
    // would move it somewhere nobody chose.
    assert_eq!(
        decide(
            &lane(Autonomy::Auto, Some("col_next")),
            &Verdict::SendBack("nope".to_owned()),
            None
        ),
        Move::Stay
    );
}

// ── Manual is the default, and it means nothing moves ─────────────────

#[test]
fn a_manual_lane_never_advances_a_card() {
    for verdict in [Verdict::Passed, Verdict::NoOpinion] {
        assert_eq!(
            decide(
                &lane(Autonomy::Manual, Some("col_next")),
                &verdict,
                Some("col_before")
            ),
            Move::Stay,
            "a manual lane moved a card forward"
        );
    }
}

#[test]
fn an_unknown_autonomy_is_manual_and_not_permission() {
    // The direction matters: a word this build does not understand must not
    // be read as consent to move somebody's card.
    for word in ["", "automatic", "AUTO", "yes", "semi"] {
        assert_eq!(
            Autonomy::parse(word),
            Autonomy::Manual,
            "{word:?} was not manual"
        );
    }
    assert_eq!(Autonomy::parse("auto"), Autonomy::Auto);
    assert_eq!(Autonomy::parse("ask"), Autonomy::Ask);
}

#[test]
fn what_was_stored_reads_back_as_itself() {
    for one in [Autonomy::Manual, Autonomy::Ask, Autonomy::Auto] {
        assert_eq!(Autonomy::parse(one.stored()), one);
    }
}

// ── Silence is not approval ───────────────────────────────────────────

#[test]
fn a_run_with_no_opinion_does_not_advance_the_card() {
    // A step that declares no verdict, or an answer that did not carry one,
    // finished — it did not approve. Reading that as a pass advances a card
    // on a judgment nobody made.
    for autonomy in [Autonomy::Ask, Autonomy::Auto] {
        assert_eq!(
            decide(
                &lane(autonomy, Some("col_next")),
                &Verdict::NoOpinion,
                Some("col_before")
            ),
            Move::Stay
        );
    }
}

#[test]
fn a_step_that_declares_nothing_gives_no_opinion() {
    assert!(matches!(
        verdict_of(&step("{}"), Some(r#"{"verdict":"ok"}"#)),
        Verdict::NoOpinion
    ));
}

#[test]
fn an_answer_missing_the_declared_field_gives_no_opinion() {
    assert!(matches!(
        verdict_of(&step(DECLARED), Some(r#"{"notes":"looks fine"}"#)),
        Verdict::NoOpinion
    ));
}

#[test]
fn a_run_that_answered_nothing_at_all_gives_no_opinion() {
    assert!(matches!(
        verdict_of(&step(DECLARED), None),
        Verdict::NoOpinion
    ));
}

#[test]
fn the_declared_field_carrying_anything_else_is_a_pass() {
    assert!(matches!(
        verdict_of(&step(DECLARED), Some(r#"{"verdict":"approve"}"#)),
        Verdict::Passed
    ));
}

#[test]
fn the_send_back_value_is_read_before_the_pass() {
    assert!(matches!(
        verdict_of(&step(DECLARED), Some(r#"{"verdict":"revise"}"#)),
        Verdict::SendBack(_)
    ));
}

// ── Forward, and what stops it ────────────────────────────────────────

#[test]
fn a_lane_with_nowhere_to_send_a_pass_keeps_the_card() {
    assert_eq!(
        decide(
            &lane(Autonomy::Auto, None),
            &Verdict::Passed,
            Some("col_before")
        ),
        Move::Stay
    );
}

#[test]
fn an_ask_lane_offers_the_advance_rather_than_taking_it() {
    assert_eq!(
        decide(
            &lane(Autonomy::Ask, Some("col_next")),
            &Verdict::Passed,
            None
        ),
        Move::Offer {
            to: "col_next".to_owned()
        }
    );
}

#[test]
fn an_auto_lane_takes_it() {
    assert_eq!(
        decide(
            &lane(Autonomy::Auto, Some("col_next")),
            &Verdict::Passed,
            None
        ),
        Move::Forward {
            to: "col_next".to_owned()
        }
    );
}

/// The one that matters most.
///
/// `irreversible` already keeps a deploy from being fired by a drag. An
/// automatic advance that walks around it is the same failure through a
/// different door — so an automatic lane offers instead of entering.
#[test]
fn nothing_enters_an_irreversible_lane_on_its_own() {
    let guarded = Lane {
        autonomy: Autonomy::Auto,
        on_pass: Some("col_deploy"),
        target_is_irreversible: true,
    };
    assert_eq!(
        decide(&guarded, &Verdict::Passed, None),
        Move::Offer {
            to: "col_deploy".to_owned()
        }
    );
}
