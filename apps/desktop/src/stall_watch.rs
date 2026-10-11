//! Says when the window stops being answered, and where it stuck.
//!
//! A reload has been seen to leave the page with every call unanswered and no
//! thread inside any command, and it never happened again while anybody was
//! looking. So three clocks are kept: the window's thread, the async runtime
//! commands start on, and the page, which calls in while it is shown. A thread
//! of its own, which needs none of them, says on stderr and in the error log
//! when one stands still, with how long ago the other two last ran.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use devpit_core::reports;

/// How long a clock may stand still before it is a stall: twice the page's
/// beat, and longer than any command the window waits on should take.
const STALL_AFTER: u64 = 12;
const LOOK_EVERY: Duration = Duration::from_secs(2);

static MAIN: AtomicU64 = AtomicU64::new(0);
static RUNTIME: AtomicU64 = AtomicU64::new(0);
static PAGE: AtomicU64 = AtomicU64::new(0);
/// What the page last said about itself; a hidden page owes no beat.
static SHOWN: AtomicBool = AtomicBool::new(false);

/// What one look at a clock has to say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Said {
    Stuck(u64),
    Back(u64),
}

/// One clock's stall, told once when it starts and once when it ends.
#[derive(Debug, Default)]
pub(crate) struct Clock {
    since: Option<u64>,
}

impl Clock {
    /// `last` is when the clock last ran; `owed` is whether it should have.
    pub(crate) fn look(&mut self, now: u64, last: u64, owed: bool) -> Option<Said> {
        let still = owed && now.saturating_sub(last) > STALL_AFTER;
        match (self.since, still) {
            (None, true) => {
                self.since = Some(last);
                Some(Said::Stuck(now.saturating_sub(last)))
            }
            // Not owed any more without having run, as a page hidden mid-stall:
            // nothing ended that anybody saw end.
            (Some(from), false) => {
                self.since = None;
                (last > from).then(|| Said::Back(last - from))
            }
            _ => None,
        }
    }
}

/// Whether the watcher itself slept through its looks — a suspended machine,
/// whose clocks all read as stalls the moment it wakes.
pub(crate) fn overslept(now: u64, looked: u64) -> bool {
    now.saturating_sub(looked) > 3 * LOOK_EVERY.as_secs()
}

/// `window.beat` — the page is alive, its calls get through, and whether it
/// is shown. It calls every few seconds while it is.
#[tauri::command]
#[specta::specta]
pub fn window_beat(webview: tauri::Webview, shown: bool) {
    if webview.label() == "main" {
        PAGE.store(reports::now(), Ordering::Relaxed);
        SHOWN.store(shown, Ordering::Relaxed);
    }
}

/// The watcher. Started once, in `setup`.
pub(crate) fn watch(app: tauri::AppHandle) {
    let now = reports::now();
    for clock in [&MAIN, &RUNTIME, &PAGE] {
        clock.store(now, Ordering::Relaxed);
    }
    let _ = std::thread::Builder::new()
        .name("stall-watch".to_owned())
        .spawn(move || {
            let (mut main, mut runtime, mut page) =
                (Clock::default(), Clock::default(), Clock::default());
            let mut looked = reports::now();
            loop {
                // Stamped by the thing being watched, so a stuck one never stamps.
                let _ = app.run_on_main_thread(|| MAIN.store(reports::now(), Ordering::Relaxed));
                tauri::async_runtime::spawn(async {
                    RUNTIME.store(reports::now(), Ordering::Relaxed)
                });
                std::thread::sleep(LOOK_EVERY);
                let now = reports::now();
                if overslept(now, looked) {
                    for clock in [&MAIN, &RUNTIME, &PAGE] {
                        clock.store(now, Ordering::Relaxed);
                    }
                }
                looked = now;
                let ages = ages(now);
                let shown = SHOWN.load(Ordering::Relaxed);
                tell(
                    main.look(now, MAIN.load(Ordering::Relaxed), true),
                    "the window's thread",
                    &ages,
                );
                tell(
                    runtime.look(now, RUNTIME.load(Ordering::Relaxed), true),
                    "the async runtime",
                    &ages,
                );
                tell(
                    page.look(now, PAGE.load(Ordering::Relaxed), shown),
                    "the page, while shown,",
                    &ages,
                );
            }
        });
}

fn ages(now: u64) -> String {
    let age = |clock: &AtomicU64| now.saturating_sub(clock.load(Ordering::Relaxed));
    format!(
        "window thread {}s ago, async runtime {}s ago, page {}s ago",
        age(&MAIN),
        age(&RUNTIME),
        age(&PAGE)
    )
}

fn tell(said: Option<Said>, who: &str, ages: &str) {
    let message = match said {
        Some(Said::Stuck(seconds)) => format!("{who} has not run for {seconds}s ({ages})"),
        Some(Said::Back(seconds)) => format!("{who} ran again after {seconds}s"),
        None => return,
    };
    eprintln!("devpit stall: {message}");
    reports::record(reports::Report {
        kind: "background",
        location: Some("stall_watch"),
        message: &message,
        stack: None,
    });
}

#[cfg(test)]
#[path = "stall_watch_tests.rs"]
mod tests;
