//! Ending a child, without waiting forever to be obeyed.
//!
//! `kill()` on a pty child sends `SIGHUP`, which is a request. A shell may
//! defer it while a job finishes, and a program that installed a handler may
//! ignore it outright — and the destructor that waits for the child to go
//! waits without a deadline. That is a closed tab that hangs the window.
//!
//! So: ask, wait a short while, and insist. The shape is Waku's
//! (`waku-core/src/terminal.rs:180-190`), which met the same problem from the
//! other side.

use std::process::Command;
use std::time::{Duration, Instant};

/// How long a child gets to leave politely.
///
/// Long enough for a shell to run its exit trap, short enough that nobody
/// experiences it as the window hanging. Waku settled on the same figure.
pub const GRACE: Duration = Duration::from_millis(250);

/// How a child went.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// It had already gone before being asked.
    Already,
    /// It left when asked.
    Politely,
    /// It had to be made to.
    Forced,
    /// Nothing here could tell — no pid to ask about.
    Unknown,
}

/// Asks a child to end, then makes it, then says which happened.
///
/// `ask` is what sends the first, gentle signal — the pty's own killer. It is
/// taken as a closure rather than a `ChildKiller` so a test can drive this
/// without a pty, which is the only way to exercise the branch where the
/// child ignores the request.
pub fn stop(pid: Option<u32>, grace: Duration, ask: impl FnOnce()) -> Stopped {
    let Some(pid) = pid else {
        ask();
        return Stopped::Unknown;
    };
    if !alive(pid) {
        return Stopped::Already;
    }

    ask();

    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        if !alive(pid) {
            return Stopped::Politely;
        }
        // Short enough that the common case — a child that goes at once —
        // costs a millisecond rather than the whole grace period.
        std::thread::sleep(Duration::from_millis(4));
    }

    insist(pid);
    Stopped::Forced
}

/// Whether a process is still there.
///
/// Signal 0 checks for the process without sending anything, which is the
/// documented way to ask.
///
/// A child that has died but not been reaped answers yes: a zombie still has
/// a pid, and nothing outside the parent can tell the two apart. That is
/// tolerable here only because the reader thread waits on the child the
/// moment the pty closes, so the window is milliseconds rather than the whole
/// grace period. Take that `wait` away and every close would pay 250ms.
///
/// Through the system's own `kill` rather than a crate:
/// `apps/desktop/src/in_flight.rs` made the same call for the same reason, and
/// a dependency taken for one signal is a dependency to keep in step forever.
fn alive(pid: u32) -> bool {
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn insist(pid: u32) {
    let _ = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .output();
}

#[cfg(test)]
#[path = "stopping_tests.rs"]
mod tests;
