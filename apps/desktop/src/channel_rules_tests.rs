use devpit_rpc::{ChannelEvent, QuietHours};

use super::{
    climb, defaults, escalation, grouped, ladder, local, next_rung, quiet, routed, Climb, Rung,
    ESCALATE_MINUTES,
};

const EVERY_RUNG: [Rung; 3] = [Rung::Push, Rung::Telegram, Rung::Email];

fn both() -> Vec<String> {
    vec![crate::telegram::ID.to_owned(), crate::hub::ID.to_owned()]
}

#[test]
fn a_notice_moves_from_push_to_telegram_to_email() {
    let all = ladder(&both(), &EVERY_RUNG);
    assert_eq!(all, EVERY_RUNG);
    assert_eq!(next_rung(&all, None), Some(Rung::Push));
    assert_eq!(next_rung(&all, Some(Rung::Push)), Some(Rung::Telegram));
    assert_eq!(next_rung(&all, Some(Rung::Telegram)), Some(Rung::Email));
    assert_eq!(next_rung(&all, Some(Rung::Email)), None);
}

#[test]
fn a_channel_not_set_up_or_not_ticked_is_skipped() {
    // No push device: Telegram first, then e-mail.
    let no_push = ladder(&both(), &[Rung::Telegram, Rung::Email]);
    assert_eq!(next_rung(&no_push, None), Some(Rung::Telegram));
    assert_eq!(next_rung(&no_push, Some(Rung::Telegram)), Some(Rung::Email));
    // No Telegram: push straight to e-mail.
    let no_telegram = ladder(&both(), &[Rung::Push, Rung::Email]);
    assert_eq!(next_rung(&no_telegram, Some(Rung::Push)), Some(Rung::Email));
    // Ticked for Telegram alone: nothing above it.
    let telegram = ladder(&[crate::telegram::ID.to_owned()], &EVERY_RUNG);
    assert_eq!(telegram, [Rung::Telegram]);
    assert_eq!(next_rung(&telegram, Some(Rung::Telegram)), None);
}

#[test]
fn a_notice_waits_its_minutes_on_a_channel_then_moves_on() {
    let all = ladder(&both(), &EVERY_RUNG);
    let sent = 1_000;
    assert_eq!(
        climb(false, &all, Rung::Push, sent, 10, sent + 10 * 60 - 1),
        Climb::Wait
    );
    assert_eq!(
        climb(false, &all, Rung::Push, sent, 10, sent + 10 * 60),
        Climb::Send(Rung::Telegram)
    );
    assert_eq!(
        climb(false, &all, Rung::Email, sent, 10, sent + 60 * 60),
        Climb::Stop,
        "nothing above e-mail"
    );
}

#[test]
fn acting_stops_the_notice_where_it_is() {
    let all = ladder(&both(), &EVERY_RUNG);
    assert_eq!(climb(true, &all, Rung::Push, 0, 10, 0), Climb::Stop);
    assert_eq!(climb(true, &all, Rung::Push, 0, 10, 99_999), Climb::Stop);
}

#[test]
fn only_what_devpit_sees_dealt_with_moves_on_and_each_rule_has_its_minutes() {
    let mut rules = defaults(&both());
    assert_eq!(
        escalation(&rules, ChannelEvent::SessionWaiting),
        Some(ESCALATE_MINUTES)
    );
    assert_eq!(escalation(&rules, ChannelEvent::SessionDone), None);
    for route in &mut rules.routes {
        route.escalate_after = Some(30);
    }
    assert_eq!(escalation(&rules, ChannelEvent::SessionDone), Some(30));
    // Nothing tells devpit a failed step or a restarted MCP was dealt with.
    assert_eq!(escalation(&rules, ChannelEvent::StepFailed), None);
    assert_eq!(escalation(&rules, ChannelEvent::McpRestarted), None);
}

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
