use std::path::Path;

use super::*;

fn turn(budget: Option<f64>) -> Say<'static> {
    Say {
        command: "claude",
        env: &[],
        prompt: "hi",
        cwd: Path::new("/work"),
        model: None,
        budget_usd: budget,
        session_id: None,
        fork_at: None,
        permission: None,
        settings: None,
        mcp_config: None,
        effort: None,
        control: None,
        on_session: None,
        add_dirs: &[],
        name: None,
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
        env: &[],
        prompt: "/clear",
        cwd: dir.path(),
        model: None,
        budget_usd: None,
        session_id: Some("old"),
        fork_at: None,
        permission: None,
        settings: None,
        mcp_config: None,
        effort: None,
        control: None,
        on_session: None,
        add_dirs: &[],
        name: None,
    };
    // A script written a moment ago can be "text file busy" to exec while
    // another test forks with it still open, and `say` reports any failed
    // spawn as NotInstalled. The script is there and executable, so that is
    // the only reason left: try again, a few times.
    let said = (0..5)
        .find_map(|_| {
            match crate::talk::say(&crate::driver::Claude::default(), &turn, |_| {}, |_| {}) {
                Err(crate::AgentError::NotInstalled) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    None
                }
                other => Some(other),
            }
        })
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

/// A turn runs under the account its profile names.
///
/// The environment is the only thing that says which account, and a turn
/// spawned without it answers from the default one without a word about it —
/// the same CLI, the same prompt, someone else's bill. The stand-in prints
/// what it was given, so the assertion is about the child's environment and
/// not about how this process happens to be configured.
#[cfg(unix)]
#[test]
fn a_chat_turn_runs_under_its_profile_env() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\n\
         read -r _prompt\n\
         printf '{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"%s\"}\\n' \"$DEVPIT_ACCOUNT\"\n\
         printf '{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"%s\",\"total_cost_usd\":0}\\n' \"$DEVPIT_ACCOUNT\"\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let command = cli.to_string_lossy().into_owned();
    let env = vec![(
        "DEVPIT_ACCOUNT".to_owned(),
        "the-profile-account".to_owned(),
    )];
    let turn = Say {
        command: &command,
        env: &env,
        prompt: "hi",
        cwd: dir.path(),
        model: None,
        budget_usd: None,
        session_id: None,
        fork_at: None,
        permission: None,
        settings: None,
        mcp_config: None,
        effort: None,
        control: None,
        on_session: None,
        add_dirs: &[],
        name: None,
    };
    // Same retry as the test above, for the same reason: a script written a
    // moment ago can be "text file busy" while another test forks.
    let said = (0..5)
        .find_map(|_| {
            match crate::talk::say(&crate::driver::Claude::default(), &turn, |_| {}, |_| {}) {
                Err(crate::AgentError::NotInstalled) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    None
                }
                other => Some(other),
            }
        })
        .expect("the stand-in never started")
        .expect("a turn");
    assert_eq!(
        said.session_id.as_deref(),
        Some("the-profile-account"),
        "the turn did not run under the profile's environment"
    );
}

/// A supervised turn asks through devpit's `PreToolUse` hook; without the
/// settings that carry it, the CLI has nobody to ask and refuses every write.
#[test]
fn a_turn_carries_devpits_hook_settings_as_one_word() {
    let with = Say {
        settings: Some("/home/someone/.devpit/hooks.json"),
        ..turn(None)
    };
    let flags = argv(&with);
    assert!(flags.contains(&"--settings=/home/someone/.devpit/hooks.json".to_owned()));
    assert!(!argv(&turn(None))
        .iter()
        .any(|a| a.starts_with("--settings")));
}

/// A printing turn is offered `AskUserQuestion` and `ExitPlanMode` only when
/// something answers its permission prompts (measured on 2.1.318); the hook
/// in devpit's settings is what answers them.
#[test]
fn a_turn_devpit_hears_is_offered_the_questions_it_can_answer() {
    let with = Say {
        settings: Some("/home/someone/.devpit/hooks.json"),
        ..turn(None)
    };
    let flags = argv(&with);
    let at = flags
        .iter()
        .position(|a| a == "--permission-prompt-tool")
        .expect("no prompt tool in the line");
    assert_eq!(flags[at + 1], "stdio");
    assert!(!argv(&turn(None))
        .iter()
        .any(|a| a == "--permission-prompt-tool"));
}

/// A chat has the board's tools the way a terminal does: devpit's MCP file,
/// as one word so it cannot swallow the flag after it.
#[test]
fn a_chat_turn_is_handed_devpits_tools() {
    let with = Say {
        mcp_config: Some("/home/someone/.devpit/mcp.json"),
        ..turn(None)
    };
    assert!(argv(&with).contains(&"--mcp-config=/home/someone/.devpit/mcp.json".to_owned()));
    assert!(!argv(&turn(None))
        .iter()
        .any(|a| a.starts_with("--mcp-config")));
}

#[test]
fn every_folder_it_may_reach_is_its_own_flag() {
    let dirs = ["/work/api".to_owned(), "/work/web app".to_owned()];
    let with = Say {
        add_dirs: &dirs,
        ..turn(None)
    };
    let said = argv(&with);
    assert!(said.contains(&"--add-dir=/work/api".to_owned()));
    assert!(said.contains(&"--add-dir=/work/web app".to_owned()));
    assert!(!argv(&turn(None)).iter().any(|a| a.starts_with("--add-dir")));
}

/// An orchestrator's sessions report to it by name. Unnamed, the CLI makes a
/// new one up for every process — `workspace-c9`, then `workspace-ff` — and a
/// report sent to the old one reaches nobody.
#[test]
fn a_named_turn_is_started_under_that_name() {
    let with = Say {
        name: Some("orchestrator-workspace"),
        ..turn(None)
    };
    let said = argv(&with);
    let at = said
        .iter()
        .position(|a| a == "--name")
        .expect("no name in the line");
    assert_eq!(said[at + 1], "orchestrator-workspace");
    assert!(!argv(&turn(None)).iter().any(|a| a == "--name"));
}

/// A CLI that fails before it answers — a session it cannot resume, say —
/// says why on stderr. The turn shows that line and ends as an error, rather
/// than an empty answer called "interrupted".
#[cfg(unix)]
#[test]
fn a_cli_that_fails_says_why_in_the_answer() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\nread -r _prompt\necho 'No conversation found with session ID: abc' >&2\nexit 1\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.to_string_lossy().into_owned();
    let turn = Say {
        command: &command,
        cwd: dir.path(),
        ..turn(None)
    };
    let mut heard = Vec::new();
    let said = (0..5)
        .find_map(|_| {
            match crate::talk::say(
                &crate::driver::Claude::default(),
                &turn,
                |part| heard.push(part),
                |_| {},
            ) {
                Err(crate::AgentError::NotInstalled) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    None
                }
                other => Some(other),
            }
        })
        .expect("the stand-in never started")
        .expect("a turn");
    assert!(said.end.is_error);
    let shown = format!("{heard:?}");
    assert!(shown.contains("No conversation found"), "{shown}");
}

/// A permission prompt the hook did not decide is answered on stdin; left
/// unanswered, the turn would wait on it for good.
#[cfg(unix)]
#[test]
fn a_permission_prompt_the_hook_left_is_answered_and_the_turn_goes_on() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\nread -r _prompt\n\
         printf '%s\\n' '{\"type\":\"control_request\",\"request_id\":\"r1\",\"request\":{\"subtype\":\"can_use_tool\",\"tool_name\":\"Bash\",\"input\":{},\"decision_reason\":\"This command requires approval\"}}'\n\
         read -r reply\nprintf '%s\\n' \"$reply\" > \"$(dirname \"$0\")/reply\"\n\
         printf '{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"s\",\"total_cost_usd\":0}\\n'\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.to_string_lossy().into_owned();
    let cwd = dir.path().to_path_buf();
    let (tell, hear) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let turn = Say {
            command: &command,
            cwd: &cwd,
            ..turn(None)
        };
        let said = (0..5).find_map(|_| {
            match crate::talk::say(&crate::driver::Claude::default(), &turn, |_| {}, |_| {}) {
                Err(crate::AgentError::NotInstalled) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    None
                }
                other => Some(other.is_ok()),
            }
        });
        let _ = tell.send(said);
    });
    let said = hear
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the turn waited on a prompt nobody answered");
    assert_eq!(said, Some(true));
    let reply = std::fs::read_to_string(dir.path().join("reply")).expect("a reply");
    assert!(reply.contains(r#""request_id":"r1""#), "{reply}");
    assert!(reply.contains(r#""behavior":"deny""#), "{reply}");
}

/// A CLI that writes more to stderr than a pipe holds still finishes its
/// turn: nobody reading it used to stop the CLI, and the turn with it.
#[cfg(unix)]
#[test]
fn a_cli_that_floods_stderr_still_finishes_its_turn() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\nread -r _prompt\nhead -c 200000 /dev/zero | tr '\\0' x >&2\n\
         printf '{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"s\",\"total_cost_usd\":0}\\n'\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.to_string_lossy().into_owned();
    let cwd = dir.path().to_path_buf();
    let (tell, hear) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let turn = Say {
            command: &command,
            cwd: &cwd,
            ..turn(None)
        };
        let said = (0..5).find_map(|_| {
            match crate::talk::say(&crate::driver::Claude::default(), &turn, |_| {}, |_| {}) {
                Err(crate::AgentError::NotInstalled) => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    None
                }
                other => Some(other.map(|said| said.end.is_error)),
            }
        });
        let _ = tell.send(said);
    });
    let said = hear
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("the turn hung on a full stderr pipe");
    assert!(
        !said.expect("started").expect("a turn"),
        "the turn ended as an error"
    );
}
