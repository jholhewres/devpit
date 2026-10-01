use std::sync::Mutex;

use super::*;

/// A new conversation's session is told while the turn runs, once per id —
/// a `/clear` inside the turn names a second one, and that is told too.
#[test]
fn a_session_is_told_as_soon_as_the_stream_names_it() {
    let driver = crate::driver::driver("claude").expect("claude");
    let told = Mutex::new(Vec::<String>::new());
    let tell = |id: &str| told.lock().unwrap().push(id.to_owned());
    let lines = [
        r#"{"type":"system","subtype":"init","session_id":"s1"}"#,
        r#"{"type":"assistant","uuid":"a1","session_id":"s1","message":{"content":[]}}"#,
        r#"{"type":"assistant","uuid":"a2","session_id":"s2","message":{"content":[]}}"#,
    ];
    let heard = follow(
        driver.as_ref(),
        lines.iter().map(|line| line.to_string()),
        &Control::default(),
        Some(&tell),
        |_| {},
    );
    assert_eq!(*told.lock().unwrap(), ["s1", "s2"]);
    assert_eq!(heard.session_id.as_deref(), Some("s2"));
}

/// A message queued behind a turn goes in as soon as the agent is listening,
/// not once the background tasks it started have finished.
#[test]
fn a_resident_turn_returns_at_its_result_while_a_task_still_runs() {
    let driver = crate::driver::driver("claude").expect("claude");
    let lines = [
        r#"{"type":"system","subtype":"task_started","task_id":"t1","description":"watch"}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01}"#,
        r#"{"type":"system","subtype":"task_notification","task_id":"t1","status":"completed"}"#,
    ];
    let mut read = 0;
    let (heard, open) = next_turn(
        driver.as_ref(),
        lines
            .iter()
            .map(|line| line.to_string())
            .inspect(|_| read += 1),
        &Control::default(),
        None,
        |_| {},
    );
    assert!(open);
    assert!(heard.ended.is_some());
    assert_eq!(read, 2, "the task's later line is the next turn's to read");
}
