//! What a closed terminal ends, tested against real processes where the
//! question is what a signal did.

use super::*;

fn proc(pid: u32, ppid: u32, sid: u32) -> Proc {
    Proc { pid, ppid, sid }
}

#[test]
fn a_stat_line_is_read_past_a_name_with_spaces_and_parentheses() {
    let stat = "4242 (my (odd) name) S 4000 4242 4100 34816 4242 4194560 0 0";
    assert_eq!(parse_stat(stat), Some(proc(4242, 4000, 4100)));
    assert_eq!(
        parse_stat("7 (gone) Z 1 7 7 0"),
        None,
        "a zombie is not alive"
    );
}

#[test]
fn a_shell_owns_its_session_and_whatever_left_it() {
    let (shell, me) = (100, 999);
    let table = [
        proc(shell, 50, shell),
        // A background job, in the shell's session.
        proc(101, shell, shell),
        // A child of an agent that called `setsid`: its own session, and a
        // grandchild of that one, which names only its parent.
        proc(102, 101, 102),
        proc(103, 102, 102),
        // Someone else's terminal, and this app.
        proc(200, 1, 200),
        proc(me, 1, shell),
    ];
    let mut found = members_of(&table, &[shell], me);
    found.sort_unstable();
    assert_eq!(found, vec![101, 102, 103]);
}

#[test]
fn nothing_is_asked_of_a_shell_with_nothing_under_it() {
    assert!(members_of(&[proc(100, 50, 100)], &[100], 999).is_empty());
    assert_eq!(stop_members(&[], AGENT_GRACE), Stopped::Already);
}

/// The case that leaked: a job sent to the background, and one that left the
/// session with `setsid`, both still running under a shell at its prompt.
#[cfg(target_os = "linux")]
#[test]
fn a_background_job_and_a_setsid_child_are_both_ended() {
    use std::os::unix::process::CommandExt;

    let mut shell = std::process::Command::new("sh");
    shell
        .arg("-c")
        .arg("sleep 30 & setsid sleep 31 & wait")
        .stdout(std::process::Stdio::null());
    // SAFETY: `setsid` is async-signal-safe, which is all `pre_exec` asks.
    unsafe {
        shell.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }
    let mut shell = shell.spawn().expect("spawn");
    let leader = shell.id();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut found = Vec::new();
    while found.len() < 2 && Instant::now() < deadline {
        found = members(&[leader]).expect("a process table");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(found.len(), 2, "both sleeps belong to the shell: {found:?}");

    assert_eq!(stop_members(&found, AGENT_GRACE), Stopped::Politely);
    for pid in &found {
        assert!(!living(*pid), "{pid} outlived its terminal");
    }
    let _ = shell.kill();
    let _ = shell.wait();
}

#[test]
fn a_panes_shell_is_the_daemons_child_and_never_a_daemon_that_left_the_pane() {
    let (daemon, shell, left) = (40, 100, 200);
    let table = [
        proc(daemon, 1, daemon),
        proc(shell, daemon, shell),
        proc(101, shell, shell),
        // An agent's `setsid` child: under the shell, so one of its members.
        proc(102, 101, 102),
        // A tmux server started in the pane: reparented, still carrying.
        proc(left, 1, left),
        proc(201, left, 201),
    ];
    let carrying = [shell, 101, 102, left, 201];
    let carry = |pid: u32| carrying.contains(&pid);
    assert_eq!(shells_of(&table, daemon, carry), vec![shell]);
    assert!(shells_of(&table, daemon, |_| false).is_empty());
}

#[cfg(target_os = "linux")]
#[test]
fn a_shell_is_found_by_the_name_in_its_environment_and_a_daemon_it_started_is_not() {
    let value = format!("leaf_{}", std::process::id());
    let mut shell = std::process::Command::new("sh")
        .arg("-c")
        .arg("setsid -f sleep 30; sleep 30 & wait")
        .env("DEVPIT_TEST_PANE", &value)
        .stdout(std::process::Stdio::null())
        .spawn()
        .expect("spawn");

    // The shell, its sleep and the reparented one all carry the name.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut carrying = Vec::new();
    while carrying.len() != 3 && Instant::now() < deadline {
        carrying = started_with("DEVPIT_TEST_PANE", &value).expect("a process table");
        std::thread::sleep(Duration::from_millis(20));
    }
    let found = started_under(std::process::id(), "DEVPIT_TEST_PANE", &value);
    let nobody = started_under(std::process::id(), "DEVPIT_TEST_PANE", "leaf_nobody");
    stop_members(&carrying, AGENT_GRACE);
    let _ = shell.kill();
    let _ = shell.wait();

    assert_eq!(carrying.len(), 3, "{carrying:?}");
    assert_eq!(found, Some(vec![shell.id()]));
    assert_eq!(nobody, Some(vec![]));
}

#[cfg(target_os = "linux")]
#[test]
fn a_process_says_the_value_it_was_started_with_and_nothing_for_a_name_it_lacks() {
    let mut child = std::process::Command::new("sleep")
        .arg("30")
        .env("DEVPIT_TEST_PANE", "leaf_started_in")
        .env("DEVPIT_TEST_PANE_TOO", "leaf_not_this_one")
        .spawn()
        .expect("spawn");
    let said = environ_of(child.id(), "DEVPIT_TEST_PANE");
    let lacking = environ_of(child.id(), "DEVPIT_TEST_NOT_SET");
    let _ = child.kill();
    let _ = child.wait();

    assert_eq!(said.as_deref(), Some("leaf_started_in"));
    assert_eq!(lacking, None);
}
