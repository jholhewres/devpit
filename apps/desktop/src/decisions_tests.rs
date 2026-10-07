use super::{reachable_address, trial};

#[test]
fn a_key_goes_only_over_https_or_to_this_machine() {
    assert!(reachable_address(""));
    assert!(reachable_address("https://openrouter.ai/api/v1"));
    assert!(reachable_address("http://127.0.0.1:4010/api/v1"));
    assert!(reachable_address("http://localhost:4010"));
    assert!(!reachable_address("http://openrouter.ai/api/v1"));
    assert!(!reachable_address("ftp://example.com"));
}

#[test]
fn the_test_asks_one_question_its_rubric_can_judge() {
    let rubric = trial();
    let ids: Vec<&String> = rubric
        .questions
        .as_object()
        .expect("questions")
        .keys()
        .collect();
    assert_eq!(ids, ["blue"]);
    assert!(rubric.thresholds.contains_key("blue"));
}
