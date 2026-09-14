use super::*;

fn task(id: &str, status: &str) -> Part {
    Part::Task {
        task_id: id.to_owned(),
        call_id: None,
        task_kind: Some("local_bash".to_owned()),
        description: None,
        status: status.to_owned(),
        summary: None,
    }
}

/// The shape the CLI's schema accepts, and the one the probe sent.
#[test]
fn stopping_a_task_is_a_control_request_naming_it() {
    let sent: serde_json::Value =
        serde_json::from_str(&stop_task_request("bxk020dqw")).expect("json");
    assert_eq!(sent["type"], "control_request");
    assert_eq!(sent["request"]["subtype"], "stop_task");
    assert_eq!(sent["request"]["task_id"], "bxk020dqw");
    assert!(sent["request_id"].as_str().is_some_and(|id| !id.is_empty()));
}

#[test]
fn a_task_runs_from_its_start_until_any_other_status() {
    let mut running = Running::default();
    running.saw(&task("a", "started"));
    running.saw(&task("a", "running"));
    assert!(!running.idle());
    // `killed` and `stopped` are what a stop_task produced; `completed` is the
    // ordinary ending. All of them end it.
    running.saw(&task("a", "killed"));
    assert!(running.idle());
}

#[test]
fn stdin_closes_only_after_a_result_with_nothing_running() {
    let mut running = Running::default();
    assert!(!may_close(false, &running));
    running.saw(&task("a", "started"));
    // A result with a background task still going: closing now could cut it off
    // before the wake-up pass it would cause.
    assert!(!may_close(true, &running));
    running.saw(&task("a", "completed"));
    assert!(may_close(true, &running));
}

#[test]
fn a_control_with_no_turn_behind_it_refuses_quietly() {
    let control = Control::new();
    assert!(!control.stop_task("a"));
}
