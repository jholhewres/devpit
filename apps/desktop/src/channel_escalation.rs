//! A notice nobody acts on moves on: push, then Telegram, then e-mail, a
//! rule's minutes apart, until the person deals with what it is about.
//!
//! The timer is here because only this machine knows whether the person
//! acted — answered the session, sent the draft, dealt with the reminder,
//! pressed a button. devpit.app relays and keeps no schedule, so while this
//! computer is off or asleep nothing moves on.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use devpit_rpc::Doing;

use crate::channel_rules::{climb, escalation, hushed, ladder, routed, Climb, Rung};
use crate::channels::{Action, Told};

/// How often notices on their way up are looked at.
const EVERY: Duration = Duration::from_secs(20);

/// What a notice is about, and so how devpit tells it was dealt with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum About {
    /// A session, while it is still as it was told: waiting, failed or done.
    Session { id: String, state: Doing },
    /// A draft, while it waits to be sent.
    Draft { profile: String, session: String },
    /// A reminder, while it has gone off and nobody dealt with it or put it off.
    Reminder { card_id: String, due_at: i64 },
}

struct Climbing {
    told: Told,
    last: Rung,
    since: i64,
}

fn climbing() -> &'static Mutex<Vec<Climbing>> {
    static CLIMBING: OnceLock<Mutex<Vec<Climbing>>> = OnceLock::new();
    CLIMBING.get_or_init(Mutex::default)
}

static WATCHING: AtomicBool = AtomicBool::new(false);

/// The rungs the person set up: Telegram once a chat is linked; push and
/// e-mail as devpit.app last said they reach, both until it has said.
pub(crate) fn set_up() -> Vec<Rung> {
    let mut up = Vec::new();
    if crate::hub::connected().is_some() {
        let reach = crate::hub::reach();
        if reach.is_none_or(|reach| reach.push) {
            up.push(Rung::Push);
        }
        if reach.is_none_or(|reach| reach.email) {
            up.push(Rung::Email);
        }
    }
    if crate::telegram::connected().is_some() {
        up.push(Rung::Telegram);
    }
    up
}

/// `told` went out on `rung` at `now`: it moves on from there, in place of an
/// earlier notice about the same thing.
pub(crate) fn start(told: Told, rung: Rung, now: i64) {
    let Some(about) = told.about.clone() else {
        return;
    };
    if let Ok(mut all) = climbing().lock() {
        all.retain(|one| one.told.about.as_ref() != Some(&about));
        all.push(Climbing {
            told,
            last: rung,
            since: now,
        });
    }
    watch();
}

/// Whether pressing `action` deals with what `told` is about.
pub(crate) fn settles(action: &Action, told: &Told) -> bool {
    match (action, &told.about) {
        (
            Action::SendDraft { profile, session },
            Some(About::Draft {
                profile: p,
                session: s,
            }),
        ) => profile == p && session == s,
        (
            Action::ReminderDone { card_id } | Action::Snooze { card_id },
            Some(About::Reminder { card_id: c, .. }),
        ) => card_id == c,
        (Action::Answer { target, .. }, _) => told.asking.as_deref() == Some(target.as_str()),
        _ => false,
    }
}

/// A button was pressed on some channel: what it was about stops moving on.
pub(crate) fn acted(action: &Action) {
    if let Ok(mut all) = climbing().lock() {
        all.retain(|one| !settles(action, &one.told));
    }
}

/// Whether what `about` names still waits on the person.
fn still_open(about: &About) -> bool {
    match about {
        About::Session { id, state } => crate::island_feed::registry()
            .lock()
            .ok()
            .and_then(|all| all.get(id).map(|one| one.state == *state))
            .unwrap_or(false),
        About::Draft { profile, session } => {
            crate::reply_drafts::drafted(profile, session).is_some()
        }
        About::Reminder { card_id, due_at } => crate::projects::store()
            .ok()
            .and_then(|store| store.reminders_pending().ok())
            .is_some_and(|rows| {
                rows.iter()
                    .any(|row| &row.card_id == card_id && row.due_at == *due_at)
            }),
    }
}

fn watch() {
    if WATCHING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(EVERY);
            if !pass() {
                break;
            }
        }
        WATCHING.store(false, Ordering::SeqCst);
        // One may have started between the last look and the flag going down.
        if climbing().lock().is_ok_and(|all| !all.is_empty()) {
            watch();
        }
    });
}

/// One look: what was dealt with stops, what waited long enough moves on.
/// Answers whether anything is still on its way up.
fn pass() -> bool {
    let now = devpit_core::reports::now() as i64;
    // Someone at devpit, or a pause: nobody is told, and nothing is dropped.
    let held = crate::pausing::paused()
        || crate::telegram::app()
            .get()
            .is_some_and(crate::island_notify::in_front);
    let rules = crate::channels::rules();
    let connected = crate::channels::connected_ids();
    let set_up = set_up();
    let offset = rules.quiet.as_ref().map_or(0, |hours| hours.offset_minutes);
    let (weekday, minute) = crate::channel_rules::local(now, offset);

    // Looked at outside the lock: whether something is open asks the store
    // and the island, and a button pressed meanwhile must not wait on that.
    let taken = climbing()
        .lock()
        .map(|mut all| std::mem::take(&mut *all))
        .unwrap_or_default();
    let mut kept = Vec::new();
    let mut due: Vec<(Rung, Told)> = Vec::new();
    for mut one in taken {
        let event = one.told.event;
        let Some(minutes) = escalation(&rules, event) else {
            continue;
        };
        let acted = !one.told.about.as_ref().is_some_and(still_open);
        if held || hushed(&rules, event, weekday, minute) {
            if !acted {
                kept.push(one);
            }
            continue;
        }
        let rungs = ladder(&routed(&rules, event, &connected, weekday, minute), &set_up);
        match climb(acted, &rungs, one.last, one.since, minutes, now) {
            Climb::Stop => {}
            Climb::Wait => kept.push(one),
            Climb::Send(rung) => {
                due.push((rung, one.told.clone()));
                one.last = rung;
                one.since = now;
                kept.push(one);
            }
        }
    }
    let busy = climbing()
        .lock()
        .map(|mut all| {
            // A newer notice about the same thing, started meanwhile, wins.
            for one in kept {
                if !all.iter().any(|newer| newer.told.about == one.told.about) {
                    all.push(one);
                }
            }
            !all.is_empty()
        })
        .unwrap_or(false);
    for rung in [Rung::Push, Rung::Telegram, Rung::Email] {
        let mine: Vec<&Told> = due
            .iter()
            .filter(|(on, _)| *on == rung)
            .map(|(_, told)| told)
            .collect();
        crate::channels::send_on(rung, &mine);
    }
    busy
}

#[cfg(test)]
mod tests {
    use super::{settles, About};
    use crate::channels::{Action, Told};

    fn told(about: Option<About>, asking: Option<&str>) -> Told {
        Told {
            event: devpit_rpc::ChannelEvent::SessionWaiting,
            line: String::new(),
            bare: String::new(),
            actions: Vec::new(),
            asking: asking.map(str::to_owned),
            about,
        }
    }

    #[test]
    fn a_button_pressed_settles_the_notice_it_was_under() {
        let draft = told(
            Some(About::Draft {
                profile: "p".to_owned(),
                session: "api".to_owned(),
            }),
            None,
        );
        let send = |session: &str| Action::SendDraft {
            profile: "p".to_owned(),
            session: session.to_owned(),
        };
        assert!(settles(&send("api"), &draft));
        assert!(!settles(&send("web"), &draft));

        let reminder = told(
            Some(About::Reminder {
                card_id: "card_1".to_owned(),
                due_at: 5,
            }),
            None,
        );
        let snooze = Action::Snooze {
            card_id: "card_1".to_owned(),
        };
        assert!(settles(&snooze, &reminder));
        assert!(!settles(&snooze, &draft));

        let waiting = told(None, Some("devpit:1.%3"));
        let answer = |target: &str| Action::Answer {
            target: target.to_owned(),
            seen: devpit_rpc::PendingPrompt {
                question: String::new(),
                options: Vec::new(),
                cursor: 0,
            },
            choice: 0,
        };
        assert!(settles(&answer("devpit:1.%3"), &waiting));
        assert!(!settles(&answer("devpit:1.%4"), &waiting));
    }
}
