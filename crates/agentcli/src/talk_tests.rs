use std::path::Path;

use super::*;

fn turn(budget: Option<f64>) -> Say<'static> {
    Say {
        command: "claude",
        prompt: "hi",
        cwd: Path::new("/work"),
        model: None,
        budget_usd: budget,
        session_id: None,
        fork_at: None,
        permission: None,
        effort: None,
        control: None,
    }
}

/// The flag the installed CLI knows. `--max-cost` is answered with
/// "error: unknown option '--max-cost'" before any work starts — measured
/// against Claude Code 2.1.270.
#[test]
fn a_capped_chat_turn_passes_the_flag_the_cli_accepts() {
    let argv = argv(&turn(Some(0.25)));
    let cap = argv
        .iter()
        .position(|a| a == "--max-budget-usd")
        .expect("no cap in the line");
    assert_eq!(argv[cap + 1], "0.25");
    assert!(!argv.iter().any(|a| a == "--max-cost"));
}

#[test]
fn an_uncapped_turn_names_no_cap() {
    assert!(!argv(&turn(None)).iter().any(|a| a.starts_with("--max")));
}

/// The session a turn ends in is the one the next turn resumes.
///
/// `/clear` starts a new session part way through a turn, so a turn can print
/// two ids. Keeping the first would resume straight back into the context
/// `/clear` dropped. Run against a stand-in for the CLI that prints both.
#[cfg(unix)]
#[test]
fn the_next_turn_resumes_the_session_this_one_ended_in() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\n\
         read -r _prompt\n\
         echo '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"old\"}'\n\
         echo '{\"type\":\"system\",\"subtype\":\"local_command\",\"content\":\"cleared\",\"session_id\":\"new\"}'\n\
         echo '{\"type\":\"assistant\",\"uuid\":\"u-2\",\"parent_tool_use_id\":null,\"session_id\":\"new\",\"message\":{\"content\":[]}}'\n\
         echo '{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"new\",\"total_cost_usd\":0}'\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let command = cli.to_string_lossy().into_owned();
    let turn = Say {
        command: &command,
        prompt: "/clear",
        cwd: dir.path(),
        model: None,
        budget_usd: None,
        session_id: Some("old"),
        fork_at: None,
        permission: None,
        effort: None,
        control: None,
    };
    // A script written a moment ago can be "text file busy" to exec while
    // another test forks with it still open, and `say` reports any failed
    // spawn as NotInstalled. The script is there and executable, so that is
    // the only reason left: try again, a few times.
    let said = (0..5)
        .find_map(
            |_| match crate::talk::say(&crate::driver::Claude, &turn, |_| {}, |_| {}) {
                Err(crate::AgentError::NotInstalled) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    None
                }
                other => Some(other),
            },
        )
        .expect("the stand-in never started")
        .expect("a turn");
    assert_eq!(said.session_id.as_deref(), Some("new"));
    // Where a rewind to this turn forks: the last message the agent wrote.
    assert_eq!(said.anchor.as_deref(), Some("u-2"));
}

#[test]
fn a_rewound_turn_forks_at_the_message_it_was_rewound_to() {
    let mut forked = turn(None);
    forked.session_id = Some("s-orig");
    forked.fork_at = Some("u-1");
    let line = argv(&forked);
    let resume = line.iter().position(|a| a == "--resume").expect("resumes");
    assert_eq!(line[resume + 1], "s-orig");
    assert!(line.iter().any(|a| a == "--fork-session"));
    assert!(line.iter().any(|a| a == "--resume-session-at=u-1"));

    // With no session there is nothing to fork from.
    let mut fresh = turn(None);
    fresh.fork_at = Some("u-1");
    assert!(!argv(&fresh).iter().any(|a| a.contains("fork")));
}
