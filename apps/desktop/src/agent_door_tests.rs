use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use super::*;

fn fresh() -> AgentHealth {
    AgentHealth {
        answering: true,
        latency_ms: None,
        checked_at: None,
        failures: 0,
        restarts: 0,
        last_restart_at: None,
        detail: None,
    }
}

#[test]
fn a_door_is_restarted_only_after_failing_checks_in_a_row() {
    let mut health = fresh();
    for _ in 1..FAILURES_BEFORE_RESTART {
        assert!(!heard(&mut health, Err("timed out".into()), 1.0));
    }
    assert!(!health.answering);
    assert!(
        heard(&mut health, Err("timed out".into()), 2.0),
        "it never restarted"
    );
    restarted(&mut health, "stuck", 3.0);
    assert_eq!((health.failures, health.restarts), (0, 1));
}

#[test]
fn one_answer_in_between_starts_the_count_again() {
    let mut health = fresh();
    for _ in 1..FAILURES_BEFORE_RESTART {
        heard(&mut health, Err("timed out".into()), 1.0);
    }
    assert!(!heard(&mut health, Ok(Duration::from_millis(4)), 2.0));
    assert_eq!(health.latency_ms, Some(4.0));
    assert!(!heard(&mut health, Err("timed out".into()), 3.0));
}

/// Answers every post the way the app does, `{"ok": …}`.
fn answering(mut stream: TcpStream) {
    std::thread::spawn(move || {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
        let mut buffer = [0u8; 4096];
        let _ = stream.read(&mut buffer);
        let body = r#"{"ok":["methods"]}"#;
        let _ = write!(
            stream,
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        );
    });
}

#[test]
fn a_stuck_door_answers_again_once_it_is_opened_again() {
    let root = tempfile::tempdir().expect("tempdir");
    let ask =
        || devpit_agentapi::client::ask(root.path(), "methods", serde_json::json!({}), root.path());

    // Every connection held and never answered, as when the MCP hung.
    let (held, keep) = std::sync::mpsc::channel::<TcpStream>();
    crate::listener::open_with(root.path(), move |stream| {
        let _ = held.send(stream);
    })
    .expect("open");
    assert!(ask().is_err(), "a stuck door answered");

    crate::listener::open_with(root.path(), answering).expect("open again");
    assert_eq!(
        ask().expect("answered after the restart"),
        serde_json::json!(["methods"])
    );
    drop(keep);
}
