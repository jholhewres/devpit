//! Ending a child, tested against children that really behave that way.

use super::*;

/// A process that does what it is told, and one that will not.
///
/// Real processes rather than a mock, because the thing under test is what a
/// signal does, and a mock would only prove that the mock was written to
/// agree with the test.
fn spawn(script: &str) -> std::process::Child {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(script)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn")
}

/// Reaps in the background, the way the real arrangement does.
///
/// Without this the child dies from the signal and stays a zombie until the
/// test process gets round to waiting on it — and a zombie answers `kill -0`
/// as though it were running. In the app the reader thread calls `wait` the
/// moment the pty closes, so the window is milliseconds; a test that skips it
/// is testing an arrangement that never happens.
fn reaped(mut child: std::process::Child) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let _ = child.wait();
    })
}

fn hangup(pid: u32) {
    let _ = Command::new("kill")
        .args(["-HUP", &pid.to_string()])
        .output();
}

#[test]
fn a_child_that_goes_when_asked_is_not_forced() {
    let child = spawn("sleep 30");
    let pid = child.id();
    let reaper = reaped(child);

    assert_eq!(stop(Some(pid), GRACE, || hangup(pid)), Stopped::Politely);
    reaper.join().expect("reaper");
}

/// The case the deadline exists for. This child traps `SIGHUP` and keeps
/// going, which is exactly what a shell running a job does — and what makes
/// a destructor that waits without a deadline hang the window.
#[test]
fn a_child_that_ignores_the_request_is_made_to_stop() {
    let child = spawn("trap '' HUP; sleep 30");
    let pid = child.id();
    let reaper = reaped(child);

    let started = std::time::Instant::now();
    assert_eq!(stop(Some(pid), GRACE, || hangup(pid)), Stopped::Forced);

    // It was forced, and it was forced soon: the whole point is that the wait
    // is bounded. Generous against a loaded machine, far under `sleep 30`.
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "waited {:?}",
        started.elapsed()
    );
    reaper.join().expect("reaper");
    assert!(!alive(pid), "it is still running after being forced");
}

/// A pane whose shell exited a moment ago is an ordinary thing to close.
#[test]
fn a_child_that_had_already_gone_is_not_signalled() {
    let mut child = spawn("true");
    let pid = child.id();
    let _ = child.wait();

    let mut asked = false;
    assert_eq!(
        stop(Some(pid), GRACE, || asked = true),
        Stopped::Already,
        "it signalled a process that was already gone"
    );
    assert!(!asked);
}

/// No pid is not a reason to skip asking. The killer still runs; this only
/// admits that nothing here can say what happened next.
#[test]
fn without_a_pid_it_still_asks_and_says_it_cannot_tell() {
    let mut asked = false;
    assert_eq!(stop(None, GRACE, || asked = true), Stopped::Unknown);
    assert!(asked, "it skipped the kill because it had no pid");
}
