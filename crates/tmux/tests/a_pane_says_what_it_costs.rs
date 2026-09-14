//! What a terminal costs, measured on a real one.
//!
//! The arithmetic is tested against values elsewhere. What is in doubt here is
//! the same thing that was in doubt for naming a pane: whether tmux's tty and
//! `ps`'s tty are the same string, whether the process the shell started is
//! reachable from the pane, and whether its children are found. A test against
//! a fixture proves none of that.

use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn a_pane_running_something_reports_memory_for_its_whole_tree() {
    if !devpit_tmux::Server::available() {
        eprintln!("skipped: tmux is not installed");
        return;
    }

    let home = tempfile::tempdir().expect("a directory");
    // A process that starts a child, because the child is the half a number
    // taken from the foreground process alone would miss.
    let socket = std::env::temp_dir().join(format!("devpit-cost-{}.sock", std::process::id()));
    let _ = Command::new("tmux")
        .args(["-S", socket.to_str().expect("utf-8"), "kill-server"])
        .output();

    let server = devpit_tmux::Server::new(socket.clone());
    server
        .ensure_session("cost_test", "leaf", home.path())
        .expect("the session");

    let idle = waiting_for_a_prompt(&server);
    assert!(
        devpit_pty::agents::idle_shell(&idle),
        "a shell at its prompt was read as {idle}"
    );

    /* Backgrounded in the pane's own shell, which is the case that matters.
    `looking` keeps only rows in the foreground process group, so a
    backgrounded child is invisible to it — and an agent's real cost is
    exactly the children that are not the thing in front of the terminal.

    Sent to the interactive shell and not through `sh -c`: a non-interactive
    shell has job control off, so `&` leaves the child in the same group and
    the case never arises. That is what the first version of this did, and
    it reported three roots and a tree of three. */
    let _ = server.send_keys("cost_test:leaf", "sleep 600 & sleep 700");
    let running = waiting_for(&server, |pane| pane.command == "sleep");

    let ttys = [running.tty.clone()];
    let front = devpit_pty::looking(&ttys);
    let in_front: Vec<u32> = devpit_pty::fronts_on(&front, &running.tty)
        .map(|one| one.pid)
        .collect();
    assert!(
        !in_front.is_empty(),
        "nothing was found in front of the pane"
    );

    let all = devpit_pty::usage::on_ttys(&ttys);
    let roots = devpit_pty::usage::on_tty(&all, &running.tty);
    assert!(
        roots.len() > in_front.len(),
        "the backgrounded job was missed: {} in front, {} on the terminal — an agent's real \
         cost is the children that are not the thing in front, and `&` puts them in another \
         process group",
        in_front.len(),
        roots.len()
    );

    let costs = devpit_pty::usage::costs(&devpit_pty::usage::tree(&roots));
    let memory: u64 = costs.iter().map(|one| one.memory).sum();
    assert!(memory > 0, "a running tree was reported as using no memory");
    assert!(
        costs.iter().all(|one| one.shared),
        "our own processes can be read proportionally, so they should be"
    );

    let _ = Command::new("tmux")
        .args(["-S", socket.to_str().expect("utf-8"), "kill-server"])
        .output();
}

fn waiting_for_a_prompt(server: &devpit_tmux::Server) -> String {
    waiting_for(server, |pane| devpit_pty::agents::idle_shell(&pane.command)).command
}

/// Polls until the pane says what is wanted, or gives up after a few seconds.
fn waiting_for(
    server: &devpit_tmux::Server,
    yet: impl Fn(&devpit_tmux::Running) -> bool,
) -> devpit_tmux::Running {
    let until = Instant::now() + Duration::from_secs(8);
    let mut last = None;
    while Instant::now() < until {
        if let Some(pane) = server
            .running("cost_test")
            .ok()
            .and_then(|panes| panes.into_iter().next())
        {
            if yet(&pane) {
                return pane;
            }
            last = Some(pane);
        }
        std::thread::sleep(Duration::from_millis(80));
    }
    panic!(
        "the pane never got there; last saw {:?}",
        last.map(|one| one.command)
    );
}
