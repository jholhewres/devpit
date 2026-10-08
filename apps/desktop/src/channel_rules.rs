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

/// Until the person says otherwise: what needs them goes everywhere, a step
/// failing and devpit's own MCP restarting go nowhere.
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
            })
            .collect(),
        quiet: None,
        group_seconds: 60,
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

/// The channels `event` goes to now, of those connected.
pub(crate) fn routed(
    rules: &ChannelRules,
    event: ChannelEvent,
    connected: &[String],
    weekday: u32,
    minute: u32,
) -> Vec<String> {
    // A reminder was set by the person for that moment: it breaks the quiet.
    let hushed = event != ChannelEvent::Reminder
        && rules
            .quiet
            .as_ref()
            .is_some_and(|hours| quiet(hours, weekday, minute));
    if hushed {
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
