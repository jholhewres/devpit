//! Listings recorded from the CLI, not written from documentation.

use super::*;

/// Recorded from the installed CLI, not written from the documentation.
const REAL: &[u8] = br#"[
      {"pid":843661,"cwd":"/home/x/p","kind":"interactive",
       "startedAt":1788877447347,"sessionId":"7ebf5e9c","name":"p-17","status":"busy"}
    ]"#;

/// Also recorded: a background session says it differently.
const BACKGROUND: &[u8] = br#"[
      {"id":"0b668877","cwd":"/tmp/p","kind":"background","startedAt":1788888277998,
       "sessionId":"0b668877-f4a7-4a71-8b7d-e39f134f028d","name":"do the thing",
       "state":"working"}
    ]"#;

/// The bug this test exists for: reading only `status` made every
/// background session unknown, and those are the ones a card is watching.
#[test]
fn a_background_session_reports_that_it_is_working() {
    let sessions = parse_list(BACKGROUND).expect("decode");
    assert_eq!(sessions[0].status, Status::Busy);
    assert_eq!(sessions[0].kind, Kind::Background);
    assert_eq!(sessions[0].short_id.as_deref(), Some("0b668877"));
}

/// A background session reports `status: idle` even while it works, so
/// reading that field first would call every one of them idle.
#[test]
fn state_is_read_before_status() {
    let listing = br#"[{"sessionId":"a","kind":"background","status":"idle","state":"working"}]"#;
    assert_eq!(parse_list(listing).expect("decode")[0].status, Status::Busy);
}

/// The state that matters most: nothing moves until a person comes back,
/// and a board that shows it as "working" is a board you stop watching.
#[test]
fn a_session_waiting_on_a_person_says_so() {
    let listing = br#"[{"sessionId":"a","kind":"background","status":"idle","state":"blocked"}]"#;
    assert_eq!(
        parse_list(listing).expect("decode")[0].status,
        Status::Blocked
    );
}

#[test]
fn a_finished_session_is_not_an_idle_one() {
    let listing = br#"[{"sessionId":"a","kind":"background","status":"idle","state":"done"}]"#;
    assert_eq!(parse_list(listing).expect("decode")[0].status, Status::Done);
}

#[test]
fn the_recorded_listing_decodes() {
    let sessions = parse_list(REAL).expect("decode");
    assert_eq!(sessions.len(), 1);
    assert_eq!(sessions[0].session_id, "7ebf5e9c");
    assert_eq!(sessions[0].status, Status::Busy);
    assert_eq!(sessions[0].kind, Kind::Interactive);
    assert_eq!(sessions[0].pid, Some(843661));
}

/// The vendor adding a state must not empty the screen.
#[test]
fn an_unknown_status_decodes_instead_of_failing() {
    let sessions = parse_list(br#"[{"sessionId":"a","status":"hibernating","kind":"orbital"}]"#)
        .expect("decode");
    assert_eq!(sessions[0].status, Status::Unknown);
    assert_eq!(sessions[0].kind, Kind::Unknown);
}

/// No sessions is a normal answer, not a failure.
#[test]
fn an_empty_listing_is_not_an_error() {
    assert!(parse_list(b"[]").expect("decode").is_empty());
}

#[test]
fn output_that_is_not_a_listing_is_reported_not_guessed() {
    assert!(matches!(
        parse_list(b"claude: command not found"),
        Err(AgentError::Unreadable(_))
    ));
}
