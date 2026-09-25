use super::*;

fn never(_: &str, _: Value) -> Result<Value, String> {
    panic!("the app should not have been asked")
}

fn reply(line: &str, ask: &dyn Fn(&str, Value) -> Result<Value, String>) -> Value {
    serde_json::from_str(&handle(line, ask).expect("an answer")).expect("json")
}

#[test]
fn the_handshake_offers_tools_and_says_what_devpit_is() {
    let said = reply(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}}"#,
        &never,
    );
    assert_eq!(said["result"]["protocolVersion"], "2025-03-26");
    assert!(said["result"]["capabilities"]["tools"].is_object());
    assert!(said["result"]["instructions"]
        .as_str()
        .unwrap_or_default()
        .contains("devpit_context"));
}

#[test]
fn a_notification_gets_no_answer() {
    assert_eq!(
        handle(
            r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
            &never
        ),
        None
    );
}

#[test]
fn every_tool_is_listed_with_a_schema() {
    let said = reply(r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#, &never);
    let tools = said["result"]["tools"].as_array().expect("tools");
    assert_eq!(tools.len(), TOOLS.len());
    assert!(tools
        .iter()
        .all(|tool| tool["inputSchema"]["type"] == "object"));
}

#[test]
fn a_call_asks_the_app_by_its_method_and_says_what_it_answered() {
    let ask = |method: &str, params: Value| -> Result<Value, String> {
        assert_eq!(method, "card");
        assert_eq!(params, json!({ "cardId": "crd_1" }));
        Ok(json!({ "title": "Fix login" }))
    };
    let said = reply(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"devpit_card","arguments":{"cardId":"crd_1"}}}"#,
        &ask,
    );
    assert_eq!(said["result"]["isError"], false);
    assert!(said["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("Fix login"));
}

/// The agent reads a refusal and carries on; it is not a broken server.
#[test]
fn a_refusal_is_the_tools_error_not_the_protocols() {
    let ask = |_: &str, _: Value| -> Result<Value, String> { Err("devpit is not open".into()) };
    let said = reply(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"devpit_board"}}"#,
        &ask,
    );
    assert!(said.get("error").is_none());
    assert_eq!(said["result"]["isError"], true);
}

#[test]
fn an_unknown_method_is_a_protocol_error() {
    let said = reply(
        r#"{"jsonrpc":"2.0","id":5,"method":"prompts/list"}"#,
        &never,
    );
    assert_eq!(said["error"]["code"], -32601);
}

#[test]
fn no_tool_answers_a_question_a_session_is_stopped_on() {
    // Answering a prompt is the person's, from the window; an agent only reads.
    assert!(!TOOLS
        .iter()
        .any(|tool| tool.method.contains("answer") || tool.method.contains("press")));
}

#[test]
fn the_board_card_and_sessions_come_with_pages_a_host_can_read() {
    let ask = |_: &str, _: Value| -> Result<Value, String> { Ok(Value::Null) };
    let listed: Value = serde_json::from_str(
        &handle(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#, &ask).expect("answer"),
    )
    .expect("json");
    let uri = |name: &str| {
        listed["result"]["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .find(|tool| tool["name"] == name)
            .and_then(|tool| {
                tool["_meta"]["ui"]["resourceUri"]
                    .as_str()
                    .map(str::to_owned)
            })
    };
    assert_eq!(
        uri("devpit_board").as_deref(),
        Some("ui://devpit/board.html")
    );
    assert_eq!(uri("devpit_card").as_deref(), Some("ui://devpit/card.html"));
    assert_eq!(
        uri("devpit_sessions").as_deref(),
        Some("ui://devpit/sessions.html")
    );
    assert_eq!(uri("devpit_comment"), None);

    let read: Value = serde_json::from_str(
        &handle(
            r#"{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"ui://devpit/board.html"}}"#,
            &ask,
        )
        .expect("answer"),
    )
    .expect("json");
    let page = &read["result"]["contents"][0];
    assert_eq!(page["mimeType"], crate::apps::APP_MIME);
    let html = page["text"].as_str().expect("html");
    assert!(html.contains("ui/initialize") && html.contains("devpit_start_session"));
    // Nothing is fetched from anywhere: no host is granted.
    assert!(!html.contains("src=\"http") && !html.contains("href=\"http"));
    assert_eq!(
        page["_meta"]["ui"]["csp"]["connectDomains"],
        serde_json::json!([])
    );

    let missing = handle(
        r#"{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"ui://devpit/nope.html"}}"#,
        &ask,
    )
    .expect("answer");
    assert!(missing.contains("-32002"));
}
