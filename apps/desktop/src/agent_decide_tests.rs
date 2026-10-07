use serde_json::json;

use super::*;
use crate::deciding::Skip;

#[test]
fn a_rubric_name_reaches_nothing_outside_the_rubrics_folder() {
    for good in ["done", "code-review", "story_refinement2"] {
        assert!(rubric_name(good), "{good}");
    }
    for bad in [
        "",
        "../done",
        "a/b",
        "a\\b",
        ".hidden",
        "done.json",
        "..",
        "a b",
    ] {
        assert!(!rubric_name(bad), "{bad}");
    }
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(rubric_named(dir.path(), "../../etc/passwd").is_err());
}

#[test]
fn the_built_in_done_rubric_judges_every_question_it_asks() {
    let dir = tempfile::tempdir().expect("tempdir");
    let done = rubric_named(dir.path(), "done").expect("built in");
    let ids: Vec<&String> = done
        .questions
        .as_object()
        .expect("questions")
        .keys()
        .collect();
    assert!(!ids.is_empty());
    assert!(ids.iter().all(|id| done.thresholds.contains_key(*id)));
    assert!(rubric_named(dir.path(), "nothing-by-this-name").is_err());
}

#[test]
fn a_project_rubric_wins_over_the_built_in_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join(".devpit").join("rubrics");
    std::fs::create_dir_all(&folder).expect("mkdir");
    std::fs::write(
        folder.join("done.json"),
        r#"{ "name": "mine", "questions": { "ok": { "type": "boolean", "instructions": "OK?" } }, "thresholds": { "ok": { "min": 0.5 } } }"#,
    )
    .expect("write");
    let rubric = rubric_named(dir.path(), "done").expect("the project's");
    assert_eq!(rubric.name, "done");
    assert!(rubric.questions.get("ok").is_some());
}

#[cfg(unix)]
#[test]
fn a_rubric_linked_from_outside_the_project_is_refused() {
    let outside = tempfile::tempdir().expect("tempdir");
    let secret = outside.path().join("secret.json");
    std::fs::write(&secret, r#"{ "questions": { "q": { "type": "noul" } } }"#).expect("write");
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join(".devpit").join("rubrics");
    std::fs::create_dir_all(&folder).expect("mkdir");
    std::os::unix::fs::symlink(&secret, folder.join("leak.json")).expect("link");
    let refused = rubric_named(dir.path(), "leak").expect_err("outside");
    assert!(refused.contains("outside the project"), "{refused}");
}

#[test]
fn more_questions_than_a_decision_takes_are_refused() {
    let many: serde_json::Map<String, Value> = (0..=MOST_QUESTIONS)
        .map(|n| {
            (
                format!("q{n}"),
                json!({ "type": "noul", "instructions": "?" }),
            )
        })
        .collect();
    assert!(counted(&Value::Object(many)).is_err());
    assert!(counted(&json!({})).is_err());
    assert!(counted(&json!({ "q": { "type": "noul" } })).is_ok());
}

#[test]
fn off_is_an_error_that_says_where_to_turn_it_on() {
    let refused = answered(Decided::Skipped(Skip::Off)).expect_err("off");
    assert_eq!(
        refused,
        "Decisions is off — set it up in Settings → Decisions"
    );
}

#[test]
fn a_cut_state_is_answered_with_a_warning() {
    let decided = |truncated| Decided::Answered {
        answers: json!({ "q": { "type": "noul", "noul": 0.7 } }),
        cost_usd: 0.001,
        latency_ms: 300,
        truncated,
    };
    let cut = answered(decided(true)).expect("answered");
    assert!(cut["warning"].as_str().is_some_and(|w| w.contains("cut")));
    assert_eq!(cut["answers"]["q"]["probability"], 0.7);
    assert!(answered(decided(false))
        .expect("answered")
        .get("warning")
        .is_none());
}

#[test]
fn a_rubric_run_says_its_outcome_and_each_verdict() {
    let rubric = rubric_named(tempfile::tempdir().expect("tempdir").path(), "done").expect("done");
    let answers: serde_json::Map<String, Value> = rubric
        .questions
        .as_object()
        .expect("questions")
        .keys()
        .map(|id| (id.clone(), json!({ "type": "noul", "noul": 0.99 })))
        .collect();
    let result = crate::deciding::gated(
        Decided::Answered {
            answers: Value::Object(answers),
            cost_usd: 0.002,
            latency_ms: 400,
            truncated: false,
        },
        &rubric,
        "shadow".to_owned(),
    );
    let said = ran(result).expect("ran");
    assert_eq!(said["outcome"], "pass");
    assert_eq!(said["verdicts"]["verified"]["verdict"], "pass");
}
