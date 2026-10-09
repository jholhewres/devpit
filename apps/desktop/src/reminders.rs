//! A card's date going off at its time: the banner, the bell, the system.
//!
//! One thread for every project, asleep until the next date nobody has been
//! reminded of — one indexed `MIN(due_at)` — and woken early by anything that
//! changes a date. No poll: with nothing to remind of it waits on the change
//! alone. While a reminder is waiting it wakes at most every half hour, which
//! is what a machine coming back from sleep needs: the clock the wait runs
//! on stops while it sleeps, and the window coming back to the front rearms
//! it too.
//!
//! A reminder only tells. It never starts a session or a step — what steps
//! may do is the board's rule, and a clock is not a person deciding.

use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

use devpit_core::{preference, ReminderRow, Store};
use devpit_rpc::{Reminder, Reminders, RpcError};
use tauri::{AppHandle, Emitter};

/// The window hears this when reminders go off, with them.
pub(crate) const FIRED: &str = "reminder:fired";
/// …and this when one was snoozed, dealt with or moved, to read them again.
pub(crate) const CHANGED: &str = "reminder:changed";
/// What the bell files a reminder under.
pub(crate) const KIND: &str = "reminder";
/// The longest the thread sleeps while a reminder is waiting.
const LONGEST_SLEEP: Duration = Duration::from_secs(30 * 60);

struct Alarm {
    rearmed: Mutex<bool>,
    wake: Condvar,
}

fn alarm() -> &'static Alarm {
    static ALARM: OnceLock<Alarm> = OnceLock::new();
    ALARM.get_or_init(|| Alarm {
        rearmed: Mutex::new(false),
        wake: Condvar::new(),
    })
}

/// Wakes the thread to look at the dates again.
pub(crate) fn rearm() {
    let alarm = alarm();
    if let Ok(mut rearmed) = alarm.rearmed.lock() {
        *rearmed = true;
        alarm.wake.notify_one();
    }
}

/// A date changed: the thread looks again and the window reads them again.
pub(crate) fn changed(app: &AppHandle) {
    rearm();
    let _ = app.emit(CHANGED, ());
}

/// Whether reminders go off. On unless the person turned them off.
pub(crate) fn enabled(store: &Store) -> bool {
    store
        .preference_flag(preference::REMINDERS)
        .ok()
        .flatten()
        .unwrap_or(true)
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}

/// How long to sleep before looking again: `None` is until something
/// changes. Off, or nothing to remind of, is that.
pub(crate) fn sleep_for(now: i64, next: Option<i64>, enabled: bool) -> Option<Duration> {
    if !enabled {
        return None;
    }
    let at = next?;
    let left = Duration::from_secs(at.saturating_sub(now).max(0) as u64);
    Some(left.min(LONGEST_SLEEP))
}

/// Starts the thread. It lives as long as the app.
pub(crate) fn watch(app: AppHandle) {
    std::thread::spawn(move || loop {
        let wait = look(&app);
        let alarm = alarm();
        let Ok(mut rearmed) = alarm.rearmed.lock() else {
            return;
        };
        while !*rearmed {
            match wait {
                None => match alarm.wake.wait(rearmed) {
                    Ok(guard) => rearmed = guard,
                    Err(_) => return,
                },
                Some(left) => match alarm.wake.wait_timeout(rearmed, left) {
                    Ok((guard, timeout)) => {
                        rearmed = guard;
                        if timeout.timed_out() {
                            break;
                        }
                    }
                    Err(_) => return,
                },
            }
        }
        *rearmed = false;
    });
}

/// Sets off what came due, and answers how long to sleep.
fn look(app: &AppHandle) -> Option<Duration> {
    let store = Store::open_default().ok()?;
    let on = enabled(&store);
    if on {
        let fired = fire(&store, now());
        if !fired.is_empty() {
            tell(app, &store, &fired);
        }
    }
    sleep_for(now(), store.next_reminder().ok().flatten(), on)
}

/// Marks what came due by `at` as reminded, files it in the bell, and
/// answers it.
pub(crate) fn fire(store: &Store, at: i64) -> Vec<ReminderRow> {
    let Ok(due) = store.reminders_due(at) else {
        return Vec::new();
    };
    for one in &due {
        let _ = store.mark_reminded(&one.card_id, at);
        let _ = store.add_notice(
            Some(&one.project_id),
            KIND,
            &format!("“{}” is due", one.title),
            None,
            Some(&one.card_id),
        );
    }
    due
}

/// The window, the bell and — when devpit is behind — the system.
fn tell(app: &AppHandle, store: &Store, fired: &[ReminderRow]) {
    let _ = app.emit(crate::notices::RANG, ());
    let _ = app.emit(FIRED, drawn(store, fired.to_vec()));
    for one in fired {
        crate::channels::emit(crate::channels::Told {
            event: devpit_rpc::ChannelEvent::Reminder,
            line: format!("Reminder: {}", one.title),
            bare: "A reminder went off.".to_owned(),
            actions: vec![
                crate::channels::Action::ReminderDone {
                    card_id: one.card_id.clone(),
                },
                crate::channels::Action::Snooze {
                    card_id: one.card_id.clone(),
                },
            ],
            asking: None,
        });
    }
    if crate::island_notify::in_front(app) {
        return;
    }
    let (title, body) = match fired {
        [one] => ("Reminder".to_owned(), one.title.clone()),
        many => (
            format!("{} reminders", many.len()),
            many.iter()
                .map(|one| one.title.as_str())
                .collect::<Vec<_>>()
                .join(" · "),
        ),
    };
    crate::island_notify::show(app, &title, &body, true);
}

/// Rows as the window draws them, with their projects' names.
pub(crate) fn drawn(store: &Store, rows: Vec<ReminderRow>) -> Reminders {
    let names: std::collections::HashMap<String, String> = store
        .projects()
        .map(|projects| projects.into_iter().map(|one| (one.id, one.name)).collect())
        .unwrap_or_default();
    Reminders {
        reminders: rows
            .into_iter()
            .map(|row| Reminder {
                project: names.get(&row.project_id).cloned(),
                card_id: row.card_id,
                project_id: row.project_id,
                title: row.title,
                due_at: row.due_at as f64,
                timed: row.timed,
            })
            .collect(),
    }
}

fn pending(store: &Store) -> Result<Reminders, RpcError> {
    Ok(drawn(store, store.reminders_pending()?))
}

/// `reminders.pending` — what went off and nobody has dealt with.
#[tauri::command]
#[specta::specta]
pub async fn reminders_pending() -> Result<Reminders, RpcError> {
    crate::off_main::blocking(|| pending(&Store::open_default()?)).await
}

/// `reminder.snooze` — the reminder goes off again at `until`.
#[tauri::command]
#[specta::specta]
pub async fn reminder_snooze(
    app: AppHandle,
    card_id: String,
    until: f64,
) -> Result<Reminders, RpcError> {
    crate::off_main::blocking(move || {
        let store = Store::open_default()?;
        snooze(&store, &card_id, until, now())?;
        changed(&app);
        pending(&store)
    })
    .await
}

/// [`reminder_snooze`]'s rule: a later moment, kept with its time.
pub(crate) fn snooze(store: &Store, card_id: &str, until: f64, now: i64) -> Result<(), RpcError> {
    if !until.is_finite() || (until as i64) <= now {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Invalid,
            "a reminder is put off to a later time",
        ));
    }
    if !store.set_card_due(card_id, Some(until as i64), true)? {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::NotFound,
            "no such card",
        ));
    }
    store.read_reminder_notices(card_id, KIND, now)?;
    Ok(())
}

/// `reminder.done` — the person dealt with it. The card keeps its date.
#[tauri::command]
#[specta::specta]
pub async fn reminder_done(app: AppHandle, card_id: String) -> Result<Reminders, RpcError> {
    crate::off_main::blocking(move || {
        let store = Store::open_default()?;
        let at = now();
        if !store.handle_reminder(&card_id, at)? {
            return Err(RpcError::new(
                devpit_rpc::ErrorCode::NotFound,
                "no such reminder",
            ));
        }
        store.read_reminder_notices(&card_id, KIND, at)?;
        changed(&app);
        pending(&store)
    })
    .await
}

/// `reminders.set` — reminders on or off. Off, the thread waits on the next
/// change and nothing goes off; on again, what came due meanwhile goes off at
/// once.
#[tauri::command]
#[specta::specta]
pub async fn reminders_set(app: AppHandle, on: bool) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        Store::open_default()?.set_preference_flag(preference::REMINDERS, on)?;
        changed(&app);
        Ok(())
    })
    .await
}

/// `reminders.rearm` — the window came back to the front: a machine that
/// slept may have passed a reminder, and the thread's clock slept with it.
#[tauri::command]
#[specta::specta]
pub fn reminders_rearm() {
    rearm();
}

#[cfg(test)]
#[path = "reminders_tests.rs"]
mod tests;
