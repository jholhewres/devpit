use devpit_rpc::{ChannelEvent, QuietHours};

use super::{defaults, grouped, local, quiet, routed};

fn connected() -> Vec<String> {
    vec!["telegram".to_owned(), "email".to_owned()]
}

#[test]
fn what_needs_the_person_goes_everywhere_by_default_and_noise_nowhere() {
    let rules = defaults(&connected());
    assert_eq!(
        routed(&rules, ChannelEvent::SessionWaiting, &connected(), 1, 600),
        connected()
    );
    assert!(routed(&rules, ChannelEvent::McpRestarted, &connected(), 1, 600).is_empty());
}

#[test]
fn a_channel_no_longer_connected_is_not_sent_to() {
    let rules = defaults(&connected());
    assert_eq!(
        routed(
            &rules,
            ChannelEvent::SessionDone,
            &["email".to_owned()],
            1,
            600
        ),
        ["email"]
    );
}

#[test]
fn quiet_hours_run_overnight_and_only_a_reminder_breaks_them() {
    let night = QuietHours {
        from: 22 * 60,
        to: 7 * 60,
        days: Vec::new(),
        offset_minutes: 0,
    };
    assert!(quiet(&night, 2, 23 * 60));
    assert!(quiet(&night, 3, 6 * 60));
    assert!(!quiet(&night, 3, 8 * 60));
    let mut rules = defaults(&connected());
    rules.quiet = Some(night);
    assert!(routed(
        &rules,
        ChannelEvent::SessionWaiting,
        &connected(),
        2,
        23 * 60
    )
    .is_empty());
    assert_eq!(
        routed(&rules, ChannelEvent::Reminder, &connected(), 2, 23 * 60),
        connected()
    );
}

#[test]
fn quiet_days_hold_the_night_to_the_day_it_began() {
    // Friday night (5) into Saturday morning: quiet; Sunday night (0) is not.
    let weekend = QuietHours {
        from: 22 * 60,
        to: 8 * 60,
        days: vec![5, 6],
        offset_minutes: 0,
    };
    assert!(quiet(&weekend, 6, 7 * 60));
    assert!(!quiet(&weekend, 0, 23 * 60));
}

#[test]
fn several_events_become_one_message() {
    assert_eq!(grouped(&[]), None);
    assert_eq!(
        grouped(&["api finished".to_owned()]).as_deref(),
        Some("api finished")
    );
    let many: Vec<String> = (1..=5).map(|n| format!("s{n} finished")).collect();
    let said = grouped(&many).expect("one");
    assert!(said.starts_with("5 things in devpit:\n• s1 finished"));
}

#[test]
fn the_local_day_and_minute_follow_the_offset() {
    // 2026-10-08 22:30 UTC was a Thursday; in UTC-3 it is 19:30 the same day.
    let at = 1_791_498_600;
    assert_eq!(local(at, 0), (4, 22 * 60 + 30));
    assert_eq!(local(at, -180), (4, 19 * 60 + 30));
    assert_eq!(local(at, 120), (5, 30));
}
