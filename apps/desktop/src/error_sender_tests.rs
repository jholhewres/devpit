//! When a report goes, and what it carries.

use super::*;

#[test]
fn nobody_at_the_window_is_away_whichever_way_they_left() {
    let presence = Presence::new(0);
    assert!(!presence.away(AWAY - 1), "just started");
    assert!(presence.away(AWAY), "in focus and untouched");

    presence.touched(1_000);
    presence.focus(false, 1_000);
    assert!(!presence.away(1_000 + AWAY - 1));
    assert!(presence.away(1_000 + AWAY), "out of focus long enough");

    presence.focus(true, 2_000);
    assert!(!presence.away(2_000 + 1), "coming back pauses it");
}

#[test]
fn idle_needs_nothing_running_as_well_as_nobody_there() {
    assert!(idle(true, 0, false));
    assert!(!idle(false, 0, false), "somebody is there");
    assert!(!idle(true, 1, false), "a run or a turn is going");
    assert!(!idle(true, 0, true), "an agent pane is working");
}

#[test]
fn a_batch_every_two_minutes_and_backoff_up_to_an_hour() {
    let mut pace = Pace::default();
    assert!(pace.due(0));
    pace.sent(0);
    assert!(!pace.due(BETWEEN - 1));
    assert!(pace.due(BETWEEN));

    pace.failed(BETWEEN);
    assert_eq!(pace.next, BETWEEN + 2 * BETWEEN);
    pace.failed(BETWEEN);
    assert_eq!(pace.next, BETWEEN + 4 * BETWEEN);
    for _ in 0..20 {
        pace.failed(BETWEEN);
    }
    assert_eq!(pace.next, BETWEEN + LONGEST_WAIT);
    pace.sent(10_000);
    assert_eq!(pace.next, 10_000 + BETWEEN, "a success resets it");
}

#[test]
fn the_answer_decides_whether_a_batch_leaves() {
    assert_eq!(outcome(Some(200)), Outcome::Sent);
    assert_eq!(outcome(Some(400)), Outcome::Refused);
    assert_eq!(outcome(Some(413)), Outcome::Refused);
    assert_eq!(outcome(Some(429)), Outcome::Later);
    assert_eq!(outcome(Some(503)), Outcome::Later);
    assert_eq!(outcome(None), Outcome::Later, "offline");
}

/// The body carries the contract's fields and the environment — nothing that
/// names a person, a machine or an install.
#[test]
fn a_report_carries_only_what_the_contract_names() {
    let entry = Entry {
        fingerprint: "3f2a9c01d4e5b6a7".into(),
        kind: "internal".into(),
        location: Some("apps/desktop/src/cards.rs:12:5".into()),
        message: "the store could not be read".into(),
        stack: None,
        count: 3,
        first_seen: 0,
        last_seen: 60,
    };
    let env = Environment {
        version: "0.1.15".into(),
        os: "linux".into(),
        arch: "x86_64".into(),
        bundle: Some("deb".into()),
    };
    let body = payload(&[entry], &env);
    let report = &body["reports"][0];
    let mut fields: Vec<_> = report
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect();
    fields.sort();
    assert_eq!(
        fields,
        [
            "arch",
            "bundle",
            "count",
            "fingerprint",
            "firstSeen",
            "kind",
            "lastSeen",
            "location",
            "message",
            "os",
            "stack",
            "version"
        ]
    );
    assert_eq!(report["firstSeen"], "1970-01-01T00:00:00Z");
    assert_eq!(report["lastSeen"], "1970-01-01T00:01:00Z");
}
