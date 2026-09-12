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

/// Ends everything in a terminal's foreground process group.
///
/// The group, not the process. Two reasons, both measured. A TUI agent
/// installs a `SIGHUP` handler so it survives a disconnected terminal — so
/// closing the tmux window, which is a `SIGHUP`, left `node` running under
/// init with nothing able to reach it. And an agent runs its tools in
/// children of its own; signalling one process leaves those orphaned.
///
/// `SIGTERM` first, because an agent asked to stop can write out what it was
/// doing. Then the same wait-and-insist as [`stop`], for the same reason: a
/// closed tab must not be able to hang the window.
pub fn stop_group(pgid: u32, grace: Duration) -> Stopped {
    if !group_alive(pgid) {
        return Stopped::Already;
    }
    signal_group(pgid, TERM);

    let deadline = Instant::now() + grace;
    while Instant::now() < deadline {
        if !group_alive(pgid) {
            return Stopped::Politely;
        }
        std::thread::sleep(Duration::from_millis(4));
    }

    signal_group(pgid, KILL);
    Stopped::Forced
}

const TERM: i32 = 15;
const KILL: i32 = 9;
/// Sends nothing; asks whether the target is there.
const NOTHING: i32 = 0;

/// Signals a whole process group, through the syscall.
///
/// Not `Command::new("kill")`, and this cost an afternoon to find. A negative
/// pid means "the group" to the *kernel*, and the shell builtin passes it
/// through — but `/usr/bin/kill` from procps, which is what `Command` execs,
/// takes `-1234` as an option, signals nothing, and **exits zero**. Measured
/// both ways on the same process: the builtin ended it, the binary reported
/// success and left it running.
///
/// So the rest of this file may spawn `kill` and be right, and this may not.
/// The dependency the module header declined to take for one signal is worth
/// taking for one that silently does nothing.
#[cfg(unix)]
fn signal_group(pgid: u32, signal: i32) -> bool {
    // SAFETY: `kill` reads no memory. A negative pid names a process group,
    // and an unknown group is an error rather than undefined behaviour.
    unsafe { libc::kill(-(pgid as i32), signal) == 0 }
}

#[cfg(not(unix))]
fn signal_group(_pgid: u32, _signal: i32) -> bool {
    // No process groups here, and no tmux to make one. The daemon is what
    // ends a pane's work on Windows.
    false
}

/// Whether anything in the group is still there.
fn group_alive(pgid: u32) -> bool {
    signal_group(pgid, NOTHING)
}

#[cfg(test)]
#[path = "stopping_tests.rs"]
mod tests;
