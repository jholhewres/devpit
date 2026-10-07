use serde_json::json;

use super::*;

fn at_least(min: f64, grey: Option<f64>) -> Threshold {
    Threshold {
        min: Some(min),
        max: None,
        grey,
    }
}

#[test]
fn a_probability_passes_at_its_minimum_greys_in_the_band_and_fails_below() {
    let threshold = at_least(0.85, Some(0.65));
    let noul = |p: f64| json!({ "type": "noul", "noul": p });
    assert_eq!(verdict(Some(&noul(0.85)), Some(&threshold)), Verdict::Pass);
    assert_eq!(verdict(Some(&noul(0.7)), Some(&threshold)), Verdict::Grey);
    assert_eq!(verdict(Some(&noul(0.65)), Some(&threshold)), Verdict::Grey);
    assert_eq!(verdict(Some(&noul(0.6)), Some(&threshold)), Verdict::Fail);
    // Without a grey band there is nothing between pass and fail.
    assert_eq!(
        verdict(Some(&noul(0.84)), Some(&at_least(0.85, None))),
        Verdict::Fail
    );
}

#[test]
fn a_score_where_lower_is_better_passes_at_or_below_its_maximum() {
    let threshold = Threshold {
        min: None,
        max: Some(3.0),
        grey: Some(4.0),
    };
    let score = |s: f64| json!({ "type": "score", "score": s });
    assert_eq!(verdict(Some(&score(2.0)), Some(&threshold)), Verdict::Pass);
    assert_eq!(verdict(Some(&score(4.0)), Some(&threshold)), Verdict::Grey);
    assert_eq!(verdict(Some(&score(5.0)), Some(&threshold)), Verdict::Fail);
}

#[test]
fn nothing_to_compare_is_left_to_a_person() {
    let threshold = at_least(0.5, None);
    assert_eq!(verdict(None, Some(&threshold)), Verdict::Grey);
    assert_eq!(
        verdict(Some(&json!({ "type": "noul" })), Some(&threshold)),
        Verdict::Grey
    );
    assert_eq!(
        verdict(Some(&json!({ "type": "noul", "noul": 0.99 })), None),
        Verdict::Grey
    );
}

#[test]
fn a_choice_is_judged_by_the_probability_of_what_it_chose() {
    let answer = json!({ "type": "choice", "choice": "bug", "probabilities": { "bug": 0.8, "feature": 0.2 } });
    assert_eq!(scalar(&answer), Some(0.8));
}

#[test]
fn the_worst_question_decides_the_outcome() {
    let rubric = Rubric {
        name: "done".into(),
        version: None,
        questions: json!({ "met": { "type": "noul" }, "tested": { "type": "noul" } }),
        thresholds: [
            ("met".to_owned(), at_least(0.8, Some(0.5))),
            ("tested".to_owned(), at_least(0.8, Some(0.5))),
        ]
        .into(),
    };
    let answers = |met: f64, tested: f64| json!({ "met": { "type": "noul", "noul": met }, "tested": { "type": "noul", "noul": tested } });
    assert_eq!(judged(&rubric, &answers(0.9, 0.9)).0, Verdict::Pass);
    assert_eq!(judged(&rubric, &answers(0.9, 0.6)).0, Verdict::Grey);
    let (outcome, verdicts) = judged(&rubric, &answers(0.6, 0.1));
    assert_eq!(outcome, Verdict::Fail);
    assert_eq!(verdicts["tested"]["verdict"], "fail");
    assert_eq!(verdicts["met"]["verdict"], "grey");
}
