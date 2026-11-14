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

/// A message steered in is heard when the CLI echoes its uuid back; one the
/// turn ended without taking in is said to be unheard, once.
#[cfg(unix)]
#[test]
fn a_steer_the_cli_echoed_is_heard_and_one_it_never_took_in_is_not() {
    let driver = crate::driver::driver("claude").expect("claude");
    let mut sink = std::process::Command::new("cat")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("cat");
    let control = Control::default();
    control.attach(sink.stdin.take().expect("stdin"));
    assert!(control.steer("also this", "u-1", "msg_1"));
    assert!(control.steer("and this", "u-2", "msg_2"));
    let lines = [
        r#"{"type":"user","isReplay":true,"uuid":"u-1","message":{"role":"user","content":[{"type":"text","text":"also this"}]}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01}"#,
    ];
    follow(
        driver.as_ref(),
        lines.iter().map(|line| line.to_string()),
        &control,
        None,
        |_| {},
    );
    assert_eq!(control.unheard(), ["msg_2"]);
    assert!(control.unheard().is_empty());
    let _ = sink.kill();
    let _ = sink.wait();
}

fn heard_of(lines: &[&str]) -> Heard {
    let driver = crate::driver::driver("claude").expect("claude");
    follow(
        driver.as_ref(),
        lines.iter().map(|line| line.to_string()),
        &Control::default(),
        None,
        |_| {},
    )
}

/// As Claude Code 2.1.325 streams a turn its account's limit refused: the
/// reset on its own line, then the error, then a failed result.
#[test]
fn a_turn_refused_by_a_rate_limit_ends_with_when_it_resets() {
    let heard = heard_of(&[
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1795143600,"rateLimitType":"five_hour"}}"#,
        r#"{"type":"assistant","error":"rate_limit","session_id":"s1","message":{"content":[{"type":"text","text":"You've hit your session limit · resets 3:50pm"}]}}"#,
        r#"{"type":"result","subtype":"success","is_error":true,"result":"You've hit your session limit · resets 3:50pm"}"#,
    ]);
    assert_eq!(
        heard.limit,
        Some(devpit_rpc::Limit {
            kind: devpit_rpc::LimitKind::RateLimit,
            resets_at: Some(1_795_143_600.0),
        })
    );
}

#[test]
fn a_provider_failing_ends_the_turn_as_an_outage_with_no_reset() {
    let heard = heard_of(&[
        r#"{"type":"assistant","error":"server_error","session_id":"s1","message":{"content":[{"type":"text","text":"API Error: 529 Overloaded"}]}}"#,
        r#"{"type":"result","subtype":"success","is_error":true,"result":"API Error: 529 Overloaded"}"#,
    ]);
    assert_eq!(
        heard.limit,
        Some(devpit_rpc::Limit {
            kind: devpit_rpc::LimitKind::Outage,
            resets_at: None,
        })
    );
}

/// Every turn hears where its window stands; only a refusal is a limit.
#[test]
fn a_window_that_still_allows_the_turn_is_no_limit() {
    let heard = heard_of(&[
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1795143600,"rateLimitType":"seven_day"}}"#,
        r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.01}"#,
    ]);
    assert_eq!(heard.limit, None);
}
