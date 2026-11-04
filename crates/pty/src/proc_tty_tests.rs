use super::*;

/// pts/8 is `136 << 8 | 8`; pts/300 spills into the next major.
const SHELL: &str = "4178508 (zsh) S 4178500 4178508 4178508 34824 4179229 4194304 0";
const AGENT: &str = "4179229 (claude (main)) S 4178508 4179229 4178508 34824 4179229 4194304 0";
const BEHIND: &str = "4179300 (sleep) S 4178508 4179300 4178508 34824 4179229 4194304 0";
const NO_TTY: &str = "1 (systemd) S 0 1 1 0 -1 4194560 0";

fn table() -> Vec<TtyStat> {
    [SHELL, AGENT, BEHIND, NO_TTY]
        .iter()
        .filter_map(|line| parse_stat(line))
        .collect()
}

#[test]
fn a_pseudo_terminal_is_named_as_ps_names_it() {
    assert_eq!(tty_name(34824).as_deref(), Some("pts/8"));
    assert_eq!(tty_name((137 << 8) | 44).as_deref(), Some("pts/300"));
    assert_eq!(tty_name(0), None);
    assert_eq!(tty_name(4 << 8 | 1), None, "a console is not a pane");
}

#[test]
fn a_name_with_parentheses_does_not_shift_the_columns() {
    let agent = parse_stat(AGENT).expect("parsed");
    assert_eq!(
        (agent.pid, agent.pgrp, agent.tpgid),
        (4179229, 4179229, 4179229)
    );
    assert_eq!(
        parse_stat(NO_TTY).expect("parsed").tpgid,
        0,
        "-1 is no group"
    );
}

#[test]
fn only_the_group_that_owns_the_terminal_is_in_front() {
    let argv = |pid: u32| Some(vec![format!("p{pid}")]);
    let fronts = fronts_in(&table(), &["/dev/pts/8".to_owned()], argv);
    let pids: Vec<u32> = fronts.iter().map(|front| front.pid).collect();
    assert_eq!(pids, vec![4179229]);
    assert_eq!(fronts[0].tty, "pts/8");
    assert!(fronts[0].resting);
}

#[test]
fn a_terminal_not_asked_about_is_left_out() {
    let argv = |_: u32| Some(vec!["x".to_owned()]);
    assert!(fronts_in(&table(), &["pts/9".to_owned()], argv).is_empty());
}

#[test]
fn what_a_terminal_costs_counts_its_background_jobs() {
    let found = on_ttys_in(&table(), &["pts/8".to_owned()]);
    let mut pids = found.get("pts/8").cloned().unwrap_or_default();
    pids.sort_unstable();
    assert_eq!(pids, vec![4178508, 4179229, 4179300]);
}

/// The real kernel, against a real pty: what `/proc` says is in front of it
/// is the program started there.
#[cfg(target_os = "linux")]
#[test]
fn the_kernel_says_what_a_real_terminal_holds() {
    use portable_pty::{native_pty_system, CommandBuilder, PtySize};
    let pair = native_pty_system()
        .openpty(PtySize::default())
        .expect("a pty");
    let mut command = CommandBuilder::new("sleep");
    command.arg("7.25");
    let mut child = pair.slave.spawn_command(command).expect("spawned");
    let pid = child.process_id().expect("a pid");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let found = loop {
        let row = super::table().into_iter().find(|row| row.pid == pid);
        if let Some(found) = row.filter(|row| row.in_front()) {
            break found;
        }
        assert!(std::time::Instant::now() < deadline, "never in front");
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    let tty = tty_name(found.tty_nr).expect("a pts");
    let fronts = fronts_in(&super::table(), &[format!("/dev/{tty}")], argv);
    let _ = child.kill();
    let front = fronts
        .iter()
        .find(|front| front.pid == pid)
        .expect("listed");
    assert_eq!(front.argv, vec!["sleep".to_owned(), "7.25".to_owned()]);
}
