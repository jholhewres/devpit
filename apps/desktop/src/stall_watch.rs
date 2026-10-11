//! Says when the window stops being answered, and where it stuck.
//!
//! A reload has been seen to leave the page with every call unanswered and no
//! thread inside any command, and it never happened again while anybody was
//! looking. So four clocks are kept: the window's thread, the async runtime
//! commands start on, the blocking pool their work runs on, and the page, which
//! calls in while it is shown. A thread of its own, which needs none of them,
//! says on stderr and in the error log when one stands still, with how long ago
//! the others last ran.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use devpit_core::reports;

/// How long a clock may stand still before it is a stall: twice the page's
/// beat, and longer than any command the window waits on should take.
const STALL_AFTER: u64 = 12;
const LOOK_EVERY: Duration = Duration::from_secs(2);

static MAIN: AtomicU64 = AtomicU64::new(0);
static RUNTIME: AtomicU64 = AtomicU64::new(0);
static BLOCKING: AtomicU64 = AtomicU64::new(0);
static PAGE: AtomicU64 = AtomicU64::new(0);
/// Each clock with what it stands for, as a stall is told.
static CLOCKS: [(&str, &AtomicU64); 4] = [
    ("the window's thread", &MAIN),
    ("the async runtime", &RUNTIME),
    ("the blocking pool", &BLOCKING),
    ("the page, while shown,", &PAGE),
];
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
    restart_clocks(reports::now());
    let _ = std::thread::Builder::new()
        .name("stall-watch".to_owned())
        .spawn(move || {
            let mut clocks: [Clock; 4] = Default::default();
            let mut looked = reports::now();
            loop {
                // Stamped by the thing being watched, so a stuck one never stamps.
                let _ = app.run_on_main_thread(|| MAIN.store(reports::now(), Ordering::Relaxed));
                tauri::async_runtime::spawn(async {
                    RUNTIME.store(reports::now(), Ordering::Relaxed)
                });
                tauri::async_runtime::spawn_blocking(|| {
                    BLOCKING.store(reports::now(), Ordering::Relaxed)
                });
                std::thread::sleep(LOOK_EVERY);
                let now = reports::now();
                if overslept(now, looked) {
                    restart_clocks(now);
                }
                looked = now;
                let ages = ages(now);
                let shown = SHOWN.load(Ordering::Relaxed);
                for ((who, last), clock) in CLOCKS.iter().zip(&mut clocks) {
                    let owed = shown || !std::ptr::eq(*last, &PAGE);
                    tell(
                        clock.look(now, last.load(Ordering::Relaxed), owed),
                        who,
                        &ages,
                    );
                }
            }
        });
}

fn restart_clocks(now: u64) {
    for (_, clock) in &CLOCKS {
        clock.store(now, Ordering::Relaxed);
    }
}

fn ages(now: u64) -> String {
    CLOCKS
        .iter()
        .map(|(who, last)| {
            let age = now.saturating_sub(last.load(Ordering::Relaxed));
            format!(
                "{} {age}s ago",
                who.trim_start_matches("the ").trim_end_matches(',')
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
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
