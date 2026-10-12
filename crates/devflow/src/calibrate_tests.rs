use super::*;

fn labelled(score: f64, outcome: &str, truth: bool) -> Labelled {
    Labelled {
        score: Some(score),
        outcome: outcome.to_owned(),
        truth,
    }
}

/// Four the person failed, scored at most 0.45, and four they passed, from
/// 0.8: every line from 0.5 to 0.8 agrees with all eight, so the middle of
/// that stretch is suggested, with grey just under it.
#[test]
fn a_clean_split_suggests_the_middle_of_the_lines_that_agree() {
    let samples = [
        (0.2, false),
        (0.3, false),
        (0.35, false),
        (0.45, false),
        (0.8, true),
        (0.85, true),
        (0.9, true),
        (0.95, true),
    ];
    let suggested = suggested(&samples).expect("enough labels");
    assert!((suggested.min - 0.65).abs() < 1e-9, "{suggested:?}");
    assert_eq!(suggested.grey, Some(0.6));
    assert!((suggested.accuracy - 1.0).abs() < 1e-9);
}

/// A rubric whose line sat too low: it passed what the person then
/// reopened. Its accuracy says so, and the line suggested is higher.
#[test]
fn a_line_set_too_low_shows_in_its_accuracy_and_the_suggestion_moves_it_up() {
    let mut all = vec![
        labelled(0.75, "pass", false),
        labelled(0.72, "pass", false),
        labelled(0.78, "pass", false),
        labelled(0.3, "fail", false),
        labelled(0.66, "grey", false),
    ];
    for score in [0.9, 0.92, 0.95, 0.97] {
        all.push(labelled(score, "pass", true));
    }
    let calibration = calibrated(&all);
    assert_eq!(calibration.labelled, 9);
    assert_eq!(calibration.grey, 1);
    assert_eq!(calibration.decided, 8);
    assert_eq!(calibration.agreed, 5);
    assert_eq!(calibration.accuracy, Some(5.0 / 8.0));
    let suggested = calibration.suggested.expect("suggested");
    assert!(
        suggested.min > 0.78 && suggested.min <= 0.9,
        "{suggested:?}"
    );
    assert!((suggested.accuracy - 1.0).abs() < 1e-9);
}

#[test]
fn too_few_labels_suggest_nothing_and_say_how_many_there_are() {
    let calibration = calibrated(&[
        labelled(0.9, "pass", true),
        labelled(0.2, "fail", true),
        Labelled {
            score: None,
            outcome: "skipped".into(),
            truth: true,
        },
    ]);
    assert_eq!(calibration.labelled, 2, "a skip was never judged");
    assert_eq!(calibration.accuracy, Some(0.5));
    assert_eq!(calibration.suggested, None);
    assert!(calibration
        .note
        .is_some_and(|note| note.starts_with("2 labelled so far")));
    assert_eq!(calibrated(&[]).accuracy, None);
}
