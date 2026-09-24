//! Sends the error reports the person switched on, slowly and only when
//! nobody is using the app.
//!
//! Idle is all of: no run, no chat turn, no agent pane working, and the window
//! out of focus or untouched for [`AWAY`]. Then one batch of [`BATCH`] every
//! [`BETWEEN`], and a person coming back pauses it. Nothing here names who
//! sent it: no install id, and no account token even when signed in.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use devpit_core::reports::{self, Entry};
use devpit_rpc::{ErrorReportsPreview, RpcError};
use tauri::Manager;

pub(crate) const BATCH: usize = 5;
pub(crate) const BETWEEN: u64 = 2 * 60;
pub(crate) const AWAY: u64 = 5 * 60;
/// Backoff doubles from [`BETWEEN`] and stops here.
pub(crate) const LONGEST_WAIT: u64 = 60 * 60;
const LOOKS_EVERY: Duration = Duration::from_secs(30);

/// Whether somebody is at the window: when it was last touched, and whether
/// and since when it has been out of focus.
pub(crate) struct Presence {
    last_touched: AtomicU64,
    focused: AtomicBool,
    unfocused_since: AtomicU64,
}

impl Presence {
    pub(crate) fn new(now: u64) -> Self {
        Self {
            last_touched: AtomicU64::new(now),
            focused: AtomicBool::new(true),
            unfocused_since: AtomicU64::new(now),
        }
    }

    pub(crate) fn touched(&self, now: u64) {
        self.last_touched.store(now, Ordering::Relaxed);
    }

    pub(crate) fn focus(&self, focused: bool, now: u64) {
        if !focused && self.focused.load(Ordering::Relaxed) {
            self.unfocused_since.store(now, Ordering::Relaxed);
        }
        self.focused.store(focused, Ordering::Relaxed);
        if focused {
            self.touched(now);
        }
    }

    /// Out of focus for [`AWAY`], or in focus and untouched for as long.
    pub(crate) fn away(&self, now: u64) -> bool {
        let untouched = now.saturating_sub(self.last_touched.load(Ordering::Relaxed)) >= AWAY;
        let unfocused = !self.focused.load(Ordering::Relaxed)
            && now.saturating_sub(self.unfocused_since.load(Ordering::Relaxed)) >= AWAY;
        untouched || unfocused
    }
}

/// Idle is away with nothing running that a report would compete with.
pub(crate) fn idle(away: bool, running: usize, working: bool) -> bool {
    away && running == 0 && !working
}

/// When the next batch may go.
#[derive(Debug, Default)]
pub(crate) struct Pace {
    next: u64,
    failures: u32,
}

impl Pace {
    pub(crate) fn due(&self, now: u64) -> bool {
        now >= self.next
    }

    pub(crate) fn sent(&mut self, now: u64) {
        self.failures = 0;
        self.next = now + BETWEEN;
    }

    /// Offline or refused for now: twice as long each time, up to an hour.
    pub(crate) fn failed(&mut self, now: u64) {
        self.failures = self.failures.saturating_add(1);
        let wait = BETWEEN.saturating_mul(1 << self.failures.min(10));
        self.next = now + wait.min(LONGEST_WAIT);
    }
}

/// What a report says about where it ran. Nothing else goes with it.
#[derive(Debug, Clone)]
pub(crate) struct Environment {
    pub version: String,
    pub os: String,
    pub arch: String,
    pub bundle: Option<String>,
}

impl Environment {
    fn here(app: &tauri::AppHandle) -> Self {
        use tauri::utils::config::BundleType;
        Self {
            version: app.package_info().version.to_string(),
            os: std::env::consts::OS.to_owned(),
            arch: std::env::consts::ARCH.to_owned(),
            bundle: tauri::utils::platform::bundle_type().map(|stamp| {
                match stamp {
                    BundleType::AppImage => "appimage",
                    BundleType::Deb => "deb",
                    BundleType::Rpm => "rpm",
                    BundleType::Dmg => "dmg",
                    _ => "other",
                }
                .to_owned()
            }),
        }
    }
}

/// The body of `POST /v1/reports`, exactly as sent — and as shown in Settings.
pub(crate) fn payload(entries: &[Entry], env: &Environment) -> serde_json::Value {
    let reports: Vec<_> = entries
        .iter()
        .map(|entry| {
            serde_json::json!({
                "fingerprint": entry.fingerprint,
                "kind": entry.kind,
                "message": entry.message,
                "location": entry.location,
                "stack": entry.stack,
                "count": entry.count,
                "firstSeen": reports::rfc3339(entry.first_seen),
                "lastSeen": reports::rfc3339(entry.last_seen),
                "version": env.version,
                "os": env.os,
                "arch": env.arch,
                "bundle": env.bundle,
            })
        })
        .collect();
    serde_json::json!({ "reports": reports })
}

/// What a batch's answer means for it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    /// Stored: it leaves the file.
    Sent,
    /// Refused as it is: it would be refused again, so it leaves too.
    Refused,
    /// Offline, throttled or the server's own trouble: kept for later.
    Later,
}

pub(crate) fn outcome(status: Option<u16>) -> Outcome {
    match status {
        Some(200..=299) => Outcome::Sent,
        Some(400 | 413 | 422) => Outcome::Refused,
        _ => Outcome::Later,
    }
}

/// A development build reports to production only when pointed somewhere on
/// purpose — its errors are the ones being written, not the ones shipped.
fn may_send() -> bool {
    !cfg!(debug_assertions) || std::env::var_os("DEVPIT_ACCOUNT_ORIGIN").is_some()
}

/// The loop. Started once, in `setup`.
pub(crate) fn watch(app: tauri::AppHandle) {
    if !may_send() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let mut pace = Pace::default();
        let env = Environment::here(&app);
        loop {
            tokio::time::sleep(LOOKS_EVERY).await;
            let now = reports::now();
            if !pace.due(now) || !idle_now(&app, now) {
                continue;
            }
            let Some(batch) = reports::with_current(|log| log.next_batch(BATCH)) else {
                continue;
            };
            if batch.is_empty() {
                continue;
            }
            match outcome(post(&payload(&batch, &env)).await) {
                Outcome::Sent | Outcome::Refused => {
                    let _ = reports::with_current(|log| log.forget_sent(&batch));
                    pace.sent(now);
                }
                Outcome::Later => pace.failed(now),
            }
        }
    });
}

fn idle_now(app: &tauri::AppHandle, now: u64) -> bool {
    let away = app
        .try_state::<Presence>()
        .is_some_and(|presence| presence.away(now));
    // Anything that cannot be read counts as busy: a report can wait.
    let running = app
        .try_state::<crate::chat::Talking>()
        .and_then(|talking| crate::update::blocking_now(&talking).ok())
        .map_or(1, |work| crate::update::blockers(&work));
    let working = crate::card_activity::registry()
        .lock()
        .map_or(true, |activities| activities.anyone_working());
    idle(away, running, working)
}

/// The status, or nothing when the server was not reached.
async fn post(body: &serde_json::Value) -> Option<u16> {
    let client = crate::account::client().ok()?;
    let response = client
        .post(format!("{}/v1/reports", crate::account::origin()))
        .json(body)
        .send()
        .await
        .ok()?;
    Some(response.status().as_u16())
}

/// `presence.seen` — the window was used. The window sends it at most once a
/// minute, which is all [`AWAY`] needs.
#[tauri::command]
#[specta::specta]
pub fn presence_seen(presence: tauri::State<'_, Presence>) -> Result<(), RpcError> {
    presence.touched(reports::now());
    Ok(())
}

/// `errors.preview` — what the next report would send, exactly.
#[tauri::command]
#[specta::specta]
pub async fn errors_preview(app: tauri::AppHandle) -> Result<ErrorReportsPreview, RpcError> {
    let env = Environment::here(&app);
    crate::off_main::blocking(move || {
        let found = reports::with_current(|log| (log.read().len(), log.next_batch(BATCH)));
        let (waiting, next) = found.unwrap_or_default();
        Ok(ErrorReportsPreview {
            waiting: waiting as u32,
            next: serde_json::to_string_pretty(&payload(&next, &env))
                .map_err(|err| RpcError::internal(err.to_string()))?,
        })
    })
    .await
}

#[cfg(test)]
#[path = "error_sender_tests.rs"]
mod tests;
