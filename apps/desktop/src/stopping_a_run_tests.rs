//! What a stop reaches, proved against real processes.
//!
//! The group is built here rather than through a shell. A shell's own reaction
//! to a signal is not the thing under test, and letting it into the test once
//! made a passing test that proved nothing: `sh` ended its background jobs on
//! its way out, so the group signal and the plain one looked identical.

use super::*;

use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

/// Waits until the process has ended, or gives up. Returns whether it went.
///
/// Reaped rather than signalled with nothing: these are this test's own
/// children, so an unreaped one answers `kill(pid, 0)` as a zombie and would
/// look alive long after it stopped.
fn went(child: &mut Child, patience: Duration) -> bool {
    let waited = Instant::now();
    while waited.elapsed() < patience {
        if matches!(child.try_wait(), Ok(Some(_)) | Err(_)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    false
}

/// A process that leads a group of its own — the shape `devpit_steps` gives a
/// command step so its work can be ended together.
///
/// `setpgid` rather than the `setsid` production uses: both make the process
/// the leader of a new group, which is the whole of what [`leads_a_group`]
/// asks, and only `setpgid` leaves the group in this session — where a second
/// process can be put into it, which is the point of the test.
fn a_run_that_leads_a_group() -> Child {
    let mut spawning = Command::new("sleep");
    spawning
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    // SAFETY: `setpgid` is async-signal-safe and names no group but its own.
    unsafe {
        spawning.pre_exec(|| match libc::setpgid(0, 0) {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
    spawning.spawn().expect("the run started")
}

/// Another process in that run's group — the `cargo`, the test binary, the
/// dev server a command step leaves behind.
fn something_under(leader: u32) -> Child {
    let mut spawning = Command::new("sleep");
    spawning
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    let group = leader as i32;
    // SAFETY: `setpgid` is async-signal-safe and names a group this test made.
    unsafe {
        spawning.pre_exec(move || match libc::setpgid(0, group) {
            -1 => Err(io::Error::last_os_error()),
            _ => Ok(()),
        });
    }
    spawning.spawn().expect("it joined the run's group")
}

/// Stopping a command step ends the work under it, not just the process the
/// run was started as.
///
/// Sabotage: make `leads_a_group` answer `false` and the second process is
/// still running while the card says the run was cancelled.
#[test]
fn stopping_a_command_step_ends_what_is_in_its_group() {
    let mut leader = a_run_that_leads_a_group();
    assert!(
        leads_a_group(leader.id()),
        "the run did not get a group of its own"
    );
    let mut under = something_under(leader.id());

    assert!(stop(leader.id()), "the signal reached nothing");

    assert!(
        went(&mut under, Duration::from_secs(3)),
        "the work under the run outlived the stop"
    );
    assert!(
        went(&mut leader, Duration::from_secs(3)),
        "the run itself outlived the stop"
    );
}

/// An agent turn is one process in devpit's own group. Signalling that group
/// would be signalling devpit, so a process that leads no group gets the plain
/// signal — and this is the test that says so.
#[test]
fn a_process_in_this_apps_group_is_signalled_alone() {
    let mut child = Command::new("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .expect("spawned");

    assert!(
        !leads_a_group(child.id()),
        "a child spawned without `setsid` should share this process's group"
    );
    assert!(stop(child.id()), "the signal reached nothing");
    let ended = child.wait().expect("waited");
    assert!(
        !ended.success(),
        "it ended on its own rather than the signal"
    );
}

/// A pid nothing answers to is a run that ended between the look and the
/// signal, which is an answer and not a crash.
#[test]
fn a_pid_that_is_gone_is_simply_not_stopped() {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("exit 0")
        .stdout(Stdio::null())
        .spawn()
        .expect("spawned");
    let pid = child.id();
    let _ = child.wait();

    assert!(!stop(pid), "a reaped process answered a signal");
}
