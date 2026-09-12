//! Read off real `ps` output, captured on the machine this runs on.

use super::*;

/// Two panes: one sitting at its prompt, one with an agent open. Captured
/// from `ps -o pid=,tty=,stat=,args= -t pts/8,pts/12`.
const LISTED: &str = "\
4178508 4178508 pts/8    Ss   -zsh
4178515 4178515 pts/12   Ss+  -zsh
4179229 4179229 pts/8    Sl+  node /home/j/.local/bin/claude --resume
";

#[test]
fn only_the_foreground_group_is_read() {
    let fronts = parse(LISTED);
    assert_eq!(
        fronts.len(),
        2,
        "a background shell was counted as in front"
    );
    assert!(fronts.iter().all(|front| front.pid != 4178508));
}

/// The whole reason this file exists: `#{pane_current_command}` says `node`
/// for every JavaScript agent, and the name is in the arguments.
#[test]
fn the_arguments_carry_the_name_the_executable_does_not() {
    let fronts = parse(LISTED);
    let front = on(&fronts, "/dev/pts/8").expect("pts/8 has something in front");
    assert_eq!(front.program(), "node");
    assert_eq!(front.argv[1], "/home/j/.local/bin/claude");
}

/// A login shell wears a leading dash, and there is no program called `-zsh`.
#[test]
fn a_login_shell_is_named_without_its_dash() {
    let fronts = parse(LISTED);
    let front = on(&fronts, "pts/12").expect("pts/12 has something in front");
    assert_eq!(front.program(), "zsh");
}

/// tmux writes `/dev/pts/8`; `ps` writes `pts/8`. Asking with either works.
#[test]
fn a_tty_is_found_with_or_without_dev() {
    let fronts = parse(LISTED);
    assert_eq!(on(&fronts, "pts/8"), on(&fronts, "/dev/pts/8"));
}

/// The bug this replaces, captured from `ps` on this machine.
///
/// Claude Code runs its MCP servers as children of itself, in its own process
/// group. Children start later and so wear higher pids, so taking the highest
/// pid named `bitbucket-mcp` — and that is what the sidebar said the pane was
/// doing while a conversation was happening in front of it.
const WITH_SERVERS: &str = "\
1326006 1326006 pts/9    Sl+  claude --permission-mode bypassPermissions
1326303 1326006 pts/9    Sl+  npm exec @atercates/bitbucket-mcp@latest
1326507 1326006 pts/9    S+   sh -c bitbucket-mcp
1326508 1326006 pts/9    Sl+  node /home/j/.npm/_npx/71ef/node_modules/.bin/bitbucket-mcp
1329304 1326006 pts/9    Sl+  node /home/j/.claude/plugins/omc/bridge/mcp-server.cjs
";

#[test]
fn an_agents_own_servers_do_not_take_its_place() {
    let fronts = parse(WITH_SERVERS);
    let front = on(&fronts, "pts/9").expect("pts/9 has something in front");
    assert_eq!(front.program(), "claude");
    assert_eq!(front.pid, 1326006, "a child was named instead of the agent");
}

/// Whatever is named, the group id is the same — which is what matters for
/// stopping, because ending a terminal's work means ending all of it.
#[test]
fn every_row_of_one_group_agrees_about_what_to_signal() {
    let fronts = parse(WITH_SERVERS);
    assert!(fronts.iter().all(|front| front.pgid == 1326006));
}

/// A pipeline puts every stage in one group led by the first of them, and the
/// first is what a person would say the terminal is running.
#[test]
fn the_first_stage_of_a_pipeline_is_the_answer() {
    let piped = "\
900 900 pts/3    S+   claude --print
901 900 pts/3    S+   tee /tmp/out
";
    let fronts = parse(piped);
    let front = on(&fronts, "pts/3").expect("pts/3 has something in front");
    assert_eq!(front.program(), "claude");
}

/// The leader is not always on the tty: a pipeline started in the background
/// is led by a subshell that has already gone. Measured, not assumed — no row
/// of this had `pid == pgid`.
#[test]
fn a_group_whose_leader_has_gone_still_names_something() {
    let orphaned = "\
2146329 2146328 pts/4    S+   sleep 3
2146330 2146328 pts/4    S+   cat
";
    let fronts = parse(orphaned);
    let front = on(&fronts, "pts/4").expect("pts/4 has something in front");
    assert_eq!(front.program(), "sleep");
}

/// A shell blocked reading the tty is at its prompt; one still running its
/// startup files is not, and typing into it loses the line.
#[test]
fn a_shell_waiting_on_the_keyboard_is_resting() {
    let starting = "700 700 pts/4    Rs+  -zsh\n";
    let waiting = "700 700 pts/4    Ss+  -zsh\n";
    assert!(!parse(starting)[0].resting);
    assert!(parse(waiting)[0].resting);
}

/// A pane whose tty has gone is not in the answer, and asking is not a panic.
#[test]
fn a_tty_with_nothing_on_it_is_nothing() {
    let fronts = parse(LISTED);
    assert!(on(&fronts, "pts/99").is_none());
}

/// `ps` that could not run, or ran and said nothing, is an empty list.
#[test]
fn nothing_to_read_is_no_processes() {
    assert!(parse("").is_empty());
    assert!(looking(&[]).is_empty());
}

/// An argument with a path in it still names its program by the last segment.
#[test]
fn a_program_is_named_by_its_last_path_segment() {
    assert_eq!(basename("/usr/local/bin/codex"), "codex");
    assert_eq!(basename("codex"), "codex");
}

/// The rule the launch path waits on, and the reason it waits twice.
#[test]
fn a_prompt_has_to_hold_before_it_is_believed() {
    let mut settling = Settling::new();
    assert!(!settling.looked(true), "one look was enough");
    assert!(settling.looked(true));
}

/// A shell that stops resting was never at a prompt, and the count starts over.
#[test]
fn a_shell_that_goes_back_to_work_resets_the_count() {
    let mut settling = Settling::new();
    settling.looked(true);
    settling.looked(false);
    assert!(
        !settling.looked(true),
        "a single look counted after a reset"
    );
}

/// Both halves of the rule: resting, and a shell.
#[test]
fn only_a_resting_shell_is_a_prompt() {
    let at_prompt = &parse("700 700 pts/4    Ss+  -zsh\n")[0];
    let starting = &parse("700 700 pts/4    Rs+  -zsh\n")[0];
    let agent = &parse("700 700 pts/4    Sl+  node /opt/claude\n")[0];

    assert!(at_a_prompt(at_prompt));
    assert!(!at_a_prompt(starting), "a shell running its startup files");
    assert!(!at_a_prompt(agent), "an agent already open");
}
