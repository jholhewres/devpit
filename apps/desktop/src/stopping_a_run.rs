//! Stopping a run's process, and everything under it when there is a group.
//!
//! A command step leads a process group of its own — `devpit_steps` puts the
//! shell in its own session at spawn so `make test` and the `cargo` and the
//! test binary under it end together. An agent turn does not: the CLI is one
//! process this app started, in this app's group, and signalling that group
//! would be signalling devpit.
//!
//! So this asks the kernel which of the two it is holding, rather than keeping
//! a second copy of the answer that could drift from the truth. A pid that
//! leads a group is a run that made one; anything else gets the plain signal
//! it has always got.

use std::time::{Duration, Instant};

/// How long a group is given to end on its own before the signal it cannot
/// catch. The same half second the runner allows its own timeout.
const GRACE: Duration = Duration::from_millis(500);

const TERM: i32 = 15;
const KILL: i32 = 9;
/// Sends nothing; asks whether the target is still there.
const NOTHING: i32 = 0;

/// Stops the run behind `pid`, returning whether the signal reached anything.
///
/// `SIGTERM` first, always: a CLI asked to stop writes its transcript on the
/// way out, and a test runner drops its lock file. The insisting second signal
/// is only for a group, where something is left to insist to.
pub(crate) fn stop(pid: u32) -> bool {
    if !leads_a_group(pid) {
        return signal(pid as i32, TERM);
    }

    let group = -(pid as i32);
    if !signal(group, TERM) {
        return false;
    }

    let waited = Instant::now();
    while waited.elapsed() < GRACE {
        if !signal(group, NOTHING) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    signal(group, KILL);
    true
}

/// Whether `pid` is the leader of a process group — which is to say, whether
/// the run made one for itself.
#[cfg(unix)]
fn leads_a_group(pid: u32) -> bool {
    // SAFETY: `getpgid` reads no memory of ours. `-1` is "no such process",
    // which is a run that ended between the look and the signal.
    let group = unsafe { libc::getpgid(pid as i32) };
    group != -1 && group == pid as i32
}

#[cfg(not(unix))]
fn leads_a_group(_pid: u32) -> bool {
    false
}

/// Signals through the syscall, never through `kill(1)`.
///
/// `crates/pty/src/stopping.rs` paid an afternoon for this: `/usr/bin/kill`
/// from procps reads `-1234` as an option, signals nothing, and exits zero.
/// A negative pid means "the group" to the kernel and to nobody in between.
#[cfg(unix)]
fn signal(target: i32, signal: i32) -> bool {
    // SAFETY: `kill` reads no memory. An unknown target is an error rather
    // than undefined behaviour.
    unsafe { libc::kill(target, signal) == 0 }
}

#[cfg(not(unix))]
fn signal(target: i32, _signal: i32) -> bool {
    std::process::Command::new("taskkill")
        .args(["/PID", &target.abs().to_string(), "/T"])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
#[path = "stopping_a_run_tests.rs"]
mod tests;
