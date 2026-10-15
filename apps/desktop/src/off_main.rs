//! Work that waits, kept off the thread the window draws on.
//!
//! A command that is a plain `fn` runs on Tauri's main thread — the one that
//! also paints the window and hands it every click. One that starts a process
//! (git, tmux, the person's own shell) holds that thread until the process
//! answers, and on a busy machine that is seconds: the window stops drawing
//! and stops taking input, and looks hung, because for that long it is.
//!
//! An `async` command runs on the async runtime instead, and anything blocking
//! inside it goes to the blocking pool, here.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use devpit_rpc::RpcError;

/// Runs `work` on the blocking pool and waits for it without holding a thread
/// the window needs.
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, RpcError> + Send + 'static,
) -> Result<T, RpcError> {
    let _working = Working::on(command_of(std::any::type_name_of_val(&work)));
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?
}

/// The command a closure was written in: `devpit_desktop::board::board_get::{{closure}}`
/// is `board_get`. Only a name, so a stall report can say it.
pub(crate) fn command_of(closure: &'static str) -> &'static str {
    let mut name = closure;
    while let Some(outer) = name.strip_suffix("::{{closure}}") {
        name = outer;
    }
    let name = name.split('<').next().unwrap_or(name);
    name.rsplit("::").next().unwrap_or(name)
}

/// How many answered commands a stall report lists, the latest first.
const ANSWERED_KEPT: usize = 12;

static NEXT: AtomicU64 = AtomicU64::new(0);
static WORKING: Mutex<Vec<(u64, &str, Instant)>> = Mutex::new(Vec::new());
static ANSWERED: Mutex<VecDeque<(&str, Instant, Duration)>> = Mutex::new(VecDeque::new());

/// One command on the blocking pool, until it answers or its caller goes.
struct Working(u64);

impl Working {
    fn on(name: &'static str) -> Self {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        if let Ok(mut working) = WORKING.lock() {
            working.push((id, name, Instant::now()));
        }
        Self(id)
    }
}

impl Drop for Working {
    fn drop(&mut self) {
        let Some((_, name, began)) = WORKING.lock().ok().and_then(|mut working| {
            let at = working.iter().position(|(id, _, _)| *id == self.0)?;
            Some(working.remove(at))
        }) else {
            return;
        };
        if let Ok(mut answered) = ANSWERED.lock() {
            answered.push_front((name, Instant::now(), began.elapsed()));
            answered.truncate(ANSWERED_KEPT);
        }
    }
}

/// The commands working now and those answered lately, with how long, for a
/// stall report: a page that stops right after an answer was handling it.
pub(crate) fn said() -> String {
    let now = Instant::now();
    let secs = |took: Duration| format!("{:.1}s", took.as_secs_f64());
    let working: Vec<String> = WORKING
        .lock()
        .map(|working| {
            working
                .iter()
                .map(|(_, name, began)| format!("{name} {}", secs(now - *began)))
                .collect()
        })
        .unwrap_or_default();
    let answered: Vec<String> = ANSWERED
        .lock()
        .map(|answered| {
            answered
                .iter()
                .map(|(name, at, took)| format!("{name} {} ago ({})", secs(now - *at), secs(*took)))
                .collect()
        })
        .unwrap_or_default();
    let listed = |all: Vec<String>| match all.is_empty() {
        true => "nothing".to_owned(),
        false => all.join(", "),
    };
    format!(
        "working on {}; answered {}",
        listed(working),
        listed(answered)
    )
}

#[cfg(test)]
#[path = "off_main_tests.rs"]
mod tests;
