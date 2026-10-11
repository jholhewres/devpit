//! Which events reach which channels, and when: the rules alone, apart from
//! sending, so each is tested by calling it.

use devpit_rpc::{ChannelEvent, ChannelRoute, ChannelRules, QuietHours};

/// Every event, in the order the screen lists them.
pub(crate) const EVENTS: [ChannelEvent; 7] = [
    ChannelEvent::SessionWaiting,
    ChannelEvent::SessionFailed,
    ChannelEvent::SessionDone,
    ChannelEvent::DraftReady,
    ChannelEvent::Reminder,
    ChannelEvent::StepFailed,
    ChannelEvent::McpRestarted,
];

/// Minutes a notice that waits on the person stays on one channel before it
/// moves on, until the person picks another.
pub(crate) const ESCALATE_MINUTES: u32 = 10;

/// Until the person says otherwise: what needs them goes everywhere — one
/// channel at a time when it waits on them — and a step failing and devpit's
/// own MCP restarting go nowhere.
pub(crate) fn defaults(connected: &[String]) -> ChannelRules {
    ChannelRules {
        routes: EVENTS
            .iter()
            .map(|&event| ChannelRoute {
                event,
                channels: match event {
                    ChannelEvent::StepFailed | ChannelEvent::McpRestarted => Vec::new(),
                    _ => connected.to_vec(),
                },
                escalate_after: matches!(
                    event,
                    ChannelEvent::SessionWaiting
                        | ChannelEvent::DraftReady
                        | ChannelEvent::Reminder
                )
                .then_some(ESCALATE_MINUTES),
            })
            .collect(),
        quiet: None,
        group_seconds: 60,
    }
}

/// Where a notice is told, in the order one nobody acts on moves along:
/// closest to hand first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Rung {
    Push,
    Telegram,
    Email,
}

/// The rungs a channel stands for: devpit.app is push and e-mail both.
fn rungs_of(channel: &str) -> &'static [Rung] {
    match channel {
        crate::telegram::ID => &[Rung::Telegram],
        crate::hub::ID => &[Rung::Push, Rung::Email],
        _ => &[],
    }
}

/// The rungs a notice may move along, lowest first: those of the channels its
/// event goes to, of those the person set up.
pub(crate) fn ladder(channels: &[String], set_up: &[Rung]) -> Vec<Rung> {
    let mut rungs: Vec<Rung> = channels
        .iter()
        .flat_map(|one| rungs_of(one).iter().copied())
        .filter(|rung| set_up.contains(rung))
        .collect();
    rungs.sort();
    rungs.dedup();
    rungs
}

/// The rung above `last` on `ladder`, or its first when nothing went yet.
pub(crate) fn next_rung(ladder: &[Rung], last: Option<Rung>) -> Option<Rung> {
    ladder.iter().copied().find(|&rung| Some(rung) > last)
}

/// Whether devpit can tell a notice about `event` was dealt with — a session
/// answered, a draft sent, a reminder done — which moving on needs.
pub(crate) fn escalates(event: ChannelEvent) -> bool {
    matches!(
        event,
        ChannelEvent::SessionWaiting
            | ChannelEvent::SessionFailed
            | ChannelEvent::SessionDone
            | ChannelEvent::DraftReady
            | ChannelEvent::Reminder
    )
}

/// The minutes a notice about `event` waits per channel, when it moves on.
pub(crate) fn escalation(rules: &ChannelRules, event: ChannelEvent) -> Option<u32> {
    rules
        .routes
        .iter()
        .find(|route| route.event == event)
        .and_then(|route| route.escalate_after)
        .filter(|_| escalates(event))
        .map(|minutes| minutes.clamp(1, 24 * 60))
}

/// What a notice moving along does now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Climb {
    /// Dealt with, or no channel left above it.
    Stop,
    Wait,
    Send(Rung),
}

/// A notice told on `last` at `since`: stops once the person acted or nothing
/// is above it, and moves on once `minutes` passed without either.
pub(crate) fn climb(
    acted: bool,
    ladder: &[Rung],
    last: Rung,
    since: i64,
    minutes: u32,
    now: i64,
) -> Climb {
    if acted {
        return Climb::Stop;
    }
    let Some(next) = next_rung(ladder, Some(last)) else {
        return Climb::Stop;
    };
    if now < since + i64::from(minutes) * 60 {
        Climb::Wait
    } else {
        Climb::Send(next)
    }
}

/// Whether `minute` of `weekday` (0 = Sunday) falls in the quiet hours.
pub(crate) fn quiet(hours: &QuietHours, weekday: u32, minute: u32) -> bool {
    let overnight = hours.from > hours.to;
    let inside = if overnight {
        minute >= hours.from || minute < hours.to
    } else {
        minute >= hours.from && minute < hours.to
    };
    // An overnight stretch after midnight belongs to the day it started.
    let day = if overnight && minute < hours.to {
        (weekday + 6) % 7
    } else {
        weekday
    };
    inside && (hours.days.is_empty() || hours.days.contains(&day))
}

/// Whether the quiet hours hold `event` back now.
pub(crate) fn hushed(rules: &ChannelRules, event: ChannelEvent, weekday: u32, minute: u32) -> bool {
    // A reminder was set by the person for that moment: it breaks the quiet.
    event != ChannelEvent::Reminder
        && rules
            .quiet
            .as_ref()
            .is_some_and(|hours| quiet(hours, weekday, minute))
}

/// The channels `event` goes to now, of those connected.
pub(crate) fn routed(
    rules: &ChannelRules,
    event: ChannelEvent,
    connected: &[String],
    weekday: u32,
    minute: u32,
) -> Vec<String> {
    if hushed(rules, event, weekday, minute) {
        return Vec::new();
    }
    rules
        .routes
        .iter()
        .find(|route| route.event == event)
        .map(|route| {
            route
                .channels
                .iter()
                .filter(|one| connected.contains(one))
                .cloned()
                .collect()
        })
        .unwrap_or_default()
}

/// One message for what was gathered: alone as it is, several as a list.
pub(crate) fn grouped(lines: &[String]) -> Option<String> {
    match lines {
        [] => None,
        [one] => Some(one.clone()),
        many => Some(format!(
            "{} things in devpit:\n{}",
            many.len(),
            many.iter()
                .map(|line| format!("• {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        )),
    }
}

/// The local weekday (0 = Sunday) and minute of the day at `epoch` seconds,
/// for a person `offset` minutes from UTC.
pub(crate) fn local(epoch: i64, offset: i32) -> (u32, u32) {
    let local = epoch + i64::from(offset) * 60;
    let days = local.div_euclid(86_400);
    // 1970-01-01 was a Thursday.
    (
        ((days + 4).rem_euclid(7)) as u32,
        (local.rem_euclid(86_400) / 60) as u32,
    )
}

#[cfg(test)]
#[path = "channel_rules_tests.rs"]
mod tests;
