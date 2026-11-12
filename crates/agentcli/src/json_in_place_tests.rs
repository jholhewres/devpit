use serde_json::json;

use super::{put, take};

#[test]
fn an_empty_file_object_gets_the_member_in_two_spaces() {
    assert_eq!(
        put("{}", "s", "k", &json!(1)).expect("put"),
        "{\n  \"s\": {\n    \"k\": 1\n  }\n}"
    );
}

#[test]
fn an_empty_outer_object_gets_the_member_on_a_line_of_its_own() {
    let text = "{\n  \"s\": {},\n  \"t\": 2\n}\n";
    assert_eq!(
        put(text, "s", "k", &json!(1)).expect("put"),
        "{\n  \"s\": {\n    \"k\": 1\n  },\n  \"t\": 2\n}\n"
    );
}

#[test]
fn a_member_already_there_gets_the_new_value_where_it_stands() {
    let text = "{\n  \"s\": {\n    \"k\": 1,\n    \"o\": 2\n  }\n}";
    assert_eq!(
        put(text, "s", "k", &json!([3])).expect("put"),
        "{\n  \"s\": {\n    \"k\": [\n      3\n    ],\n    \"o\": 2\n  }\n}"
    );
}

/// Braces and quotes inside strings are text, and serde_json reads the last
/// of two equal keys, so every copy goes.
#[test]
fn the_first_member_and_every_copy_of_a_key_are_taken_out() {
    let text = r#"{"s": {"k": 1, "o": "a,}\"b", "k": [2, {"x": "}"}]}}"#;
    assert_eq!(
        take(text, "s", "k").expect("take"),
        r#"{"s": { "o": "a,}\"b"}}"#
    );
    assert_eq!(
        take(r#"{"s": {"k": 1}}"#, "s", "k").expect("take"),
        r#"{"s": {}}"#
    );
}

#[test]
fn a_byte_order_mark_stays_and_a_one_line_file_stays_on_one_line() {
    assert_eq!(
        put("\u{feff}{\"a\": 1}", "s", "k", &json!({ "c": "x" })).expect("put"),
        "\u{feff}{\"a\": 1,\"s\": {\"k\":{\"c\":\"x\"}}}"
    );
}

#[test]
fn what_is_not_an_object_where_one_belongs_is_refused_or_left() {
    assert!(put("[1]", "s", "k", &json!(1)).is_err());
    assert!(put(r#"{"s": []}"#, "s", "k", &json!(1)).is_err());
    assert!(put("{ not json", "s", "k", &json!(1)).is_err());
    assert_eq!(
        take(r#"{"s": []}"#, "s", "k").expect("take"),
        r#"{"s": []}"#
    );
    assert!(take("[1]", "s", "k").is_err());
}
