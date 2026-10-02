//! A pause: devpit quiet for a while, for a meeting, a talk, a stretch of
//! thought.
//!
//! Quiet, not deaf. While paused, a terminal's permission question goes
//! straight to the terminal instead of waiting on an island nobody is looking
//! at; no system notification is shown; and the island neither opens nor
//! makes a sound. Hooks are still heard, so the board and the island are
//! right the moment the pause ends. Reminders still go off: their time was
//! the person's own choice.
//!
//! Kept across a restart, and it ends on its own at the time it was given.

use std::sync::atomic::{AtomicI64, Ordering};

use devpit_core::{preference, Store};
use devpit_rpc::{Paused, RpcError};
use tauri::{AppHandle, Emitter};

/// The event every window hears when the pause changes.
pub(crate) const CHANGED: &str = "pause:changed";
/// Not paused.
const OFF: i64 = 0;
/// Paused until resumed.
const UNTIL_RESUMED: i64 = i64::MAX;

static UNTIL: AtomicI64 = AtomicI64::new(OFF);

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}

/// Whether a pause ending at `until` is on at `now`.
pub(crate) fn on_at(until: i64, now: i64) -> bool {
    until != OFF && now < until
}

/// Whether devpit is paused now.
pub(crate) fn paused() -> bool {
    on_at(UNTIL.load(Ordering::Relaxed), now())
}

fn said(until: i64) -> Paused {
    let on = on_at(until, now());
    Paused {
        on,
        until: (on && until != UNTIL_RESUMED).then_some(until as f64),
    }
}

/// Reads the pause kept from before this start, and ends it at its time.
pub(crate) fn restore(app: &AppHandle, store: &Store) {
    let kept = store
        .preference(preference::PAUSED_UNTIL)
        .ok()
        .flatten()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(OFF);
    let until = if on_at(kept, now()) { kept } else { OFF };
    UNTIL.store(until, Ordering::Relaxed);
    end_when_due(app, until);
}

/// Resumes at `until`, unless the pause was changed by then. Without it the
/// pause ended only in what `paused()` answered, and the tray kept offering
/// "Resume" for a pause long over.
fn end_when_due(app: &AppHandle, until: i64) {
    if until == OFF || until == UNTIL_RESUMED {
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        let left = until.saturating_sub(now()).max(0) as u64;
        std::thread::sleep(std::time::Duration::from_secs(left));
        if UNTIL
            .compare_exchange(until, OFF, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            let _ = set(&app, OFF);
        }
    });
}

/// `pause.read` — whether devpit is paused, and until when.
#[tauri::command]
#[specta::specta]
pub fn pause_read() -> Paused {
    said(UNTIL.load(Ordering::Relaxed))
}

/// `pause.set` — pauses until `until` (seconds since the epoch), until
/// resumed when `forever`, or resumes when neither.
#[tauri::command]
#[specta::specta]
pub async fn pause_set(
    app: AppHandle,
    until: Option<f64>,
    forever: bool,
) -> Result<Paused, RpcError> {
    crate::off_main::blocking(move || {
        let until = match (forever, until) {
            (true, _) => UNTIL_RESUMED,
            (false, Some(at)) if at.is_finite() && (at as i64) > now() => at as i64,
            (false, Some(_)) => {
                return Err(RpcError::new(
                    devpit_rpc::ErrorCode::Invalid,
                    "a pause ends later than now",
                ))
            }
            (false, None) => OFF,
        };
        set(&app, until)
    })
    .await
}

/// Pauses until `until`, or resumes at `0`, and tells every window.
pub(crate) fn set(app: &AppHandle, until: i64) -> Result<Paused, RpcError> {
    UNTIL.store(until, Ordering::Relaxed);
    Store::open_default()?.set_preference(preference::PAUSED_UNTIL, &until.to_string())?;
    let now = said(until);
    let _ = app.emit(CHANGED, now);
    crate::desk::tray_refresh(app);
    end_when_due(app, until);
    Ok(now)
}

/// Paused for an hour from now: the tray's one choice.
pub(crate) fn for_an_hour(app: &AppHandle) {
    let _ = set(app, now() + 3600);
}

/// Resumed.
pub(crate) fn resume(app: &AppHandle) {
    let _ = set(app, OFF);
}

#[cfg(test)]
#[path = "pausing_tests.rs"]
mod tests;
