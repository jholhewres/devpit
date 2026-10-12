use serde_json::{json, Value};

use super::{error, id_of};

#[test]
fn a_request_is_known_by_its_id_and_a_notification_by_having_none() {
    assert_eq!(
        id_of(r#"{"jsonrpc":"2.0","id":3,"method":"ping"}"#),
        Some(json!(3))
    );
    assert_eq!(
        id_of(r#"{"jsonrpc":"2.0","id":"a","method":"ping"}"#),
        Some(json!("a"))
    );
    assert_eq!(
        id_of(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
        None
    );
    assert_eq!(id_of(r#"{"jsonrpc":"2.0","id":null}"#), None);
    assert_eq!(id_of("not json"), None);
}

#[test]
fn an_error_answers_the_request_it_is_for() {
    let said: Value = serde_json::from_str(&error(&json!(9), "gone")).expect("json");
    assert_eq!(said["id"], 9);
    assert_eq!(said["error"]["message"], "gone");
    assert!(said.get("result").is_none());
}
