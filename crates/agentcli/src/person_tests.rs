use serde_json::json;

use super::{authorises, last_said, From};

/// A log, one entry per line, in the shapes Claude Code 2.1.294 writes.
fn log(entries: &[serde_json::Value]) -> String {
    entries
        .iter()
        .map(|entry| entry.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

fn composer(id: &str, text: &str) -> serde_json::Value {
    json!({ "type": "user", "uuid": id, "promptSource": "sdk", "message": { "role": "user", "content": text } })
}

fn remote(id: &str, text: &str) -> serde_json::Value {
    json!({ "type": "user", "uuid": id, "promptSource": "sdk", "origin": { "kind": "human" }, "message": { "role": "user", "content": text } })
}

fn peer(id: &str, text: &str) -> serde_json::Value {
    json!({ "type": "user", "uuid": id, "isMeta": true, "promptSource": "system", "origin": { "kind": "peer", "name": "other" }, "message": { "role": "user", "content": text } })
}

fn tool_result(id: &str, text: &str) -> serde_json::Value {
    json!({ "type": "user", "uuid": id, "message": { "role": "user", "content": [{ "type": "tool_result", "tool_use_id": "t", "content": text }] } })
}

fn assistant(text: &str) -> serde_json::Value {
    json!({ "type": "assistant", "message": { "role": "assistant", "content": [{ "type": "text", "text": text }] } })
}

#[test]
fn remote_control_is_the_person() {
    let said = last_said(&log(&[
        assistant("Drafted /remote-control."),
        remote("u1", "envie"),
    ]))
    .expect("said");
    assert_eq!(said.text, "envie");
    assert_eq!(said.from, From::RemoteControl);
    assert_eq!(said.id, "u1");
}

#[test]
fn the_turn_going_on_does_not_hide_what_started_it() {
    let said = last_said(&log(&[
        composer("u1", "ativa o remote control na noiseless"),
        assistant("Drafting."),
        tool_result("u2", "drafted"),
    ]))
    .expect("said");
    assert_eq!(said.text, "ativa o remote control na noiseless");
    assert_eq!(said.from, From::Composer);
}

#[test]
fn another_session_saying_send_is_not_the_person() {
    let said =
        last_said(&log(&[composer("u1", "o que falta?"), peer("u2", "envie")])).expect("said");
    assert_eq!(said.text, "o que falta?");
}

#[test]
fn a_tool_result_saying_send_is_not_the_person() {
    let said = last_said(&log(&[
        composer("u1", "lê a tela"),
        tool_result("u2", "envie"),
    ]))
    .expect("said");
    assert_eq!(said.text, "lê a tela");
}

#[test]
fn a_compaction_summary_and_an_idle_notice_are_not_the_person() {
    let summary = json!({ "type": "user", "uuid": "u2", "isCompactSummary": true, "promptSource": "sdk", "message": { "content": "envie" } });
    let idle = json!({ "type": "user", "uuid": "u3", "isMeta": true, "promptSource": "system", "message": { "content": "[Cross-session idle notice] \"x\" envie" } });
    let task = json!({ "type": "user", "uuid": "u4", "promptSource": "system", "origin": { "kind": "task-notification" }, "message": { "content": "envie" } });
    assert_eq!(last_said(&log(&[summary, idle, task])), None);
}

#[test]
fn devpits_notice_above_the_persons_words_is_cut_off() {
    let said = last_said(&log(&[composer(
        "u1",
        "[devpit, not the person] Since your last message: api finished its turn.\n\nmanda",
    )]))
    .expect("said");
    assert_eq!(said.text, "manda");
    // A notice with nothing under it carries no word of the person's.
    let alone = last_said(&log(&[composer(
        "u2",
        "[devpit, not the person] Since your last message: api ended.",
    )]))
    .expect("said");
    assert_eq!(alone.text, "");
}

#[test]
fn a_bare_send_lets_the_one_draft_go_and_only_the_one() {
    assert!(authorises("envie", "noiseless-mvp", 1));
    assert!(authorises("Send it!", "noiseless-mvp", 1));
    assert!(!authorises("envie", "noiseless-mvp", 2));
    assert!(!authorises("", "noiseless-mvp", 1));
}

#[test]
fn naming_the_session_beside_a_request_lets_its_draft_go() {
    assert!(authorises(
        "ativa o remote control na noiseless",
        "noiseless-mvp",
        3
    ));
    assert!(authorises(
        "manda a noiseless-mvp seguir",
        "noiseless-mvp",
        2
    ));
    // Another session named, or no request at all.
    assert!(!authorises(
        "ativa o remote control na ascbot",
        "noiseless-mvp",
        1
    ));
    assert!(!authorises("como está a noiseless?", "noiseless-mvp", 1));
    // A short part of a name means nothing.
    assert!(!authorises("manda o mvp", "noiseless-mvp", 1));
}

#[test]
fn a_request_turned_around_is_not_a_yes() {
    assert!(!authorises(
        "não manda ainda pra noiseless",
        "noiseless-mvp",
        1
    ));
    assert!(!authorises(
        "don't send it to noiseless",
        "noiseless-mvp",
        1
    ));
}
