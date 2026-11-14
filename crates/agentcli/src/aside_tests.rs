use std::path::Path;

use super::*;

fn asked(session_id: Option<&'static str>) -> Aside<'static> {
    Aside {
        command: "claude",
        env: &[],
        cwd: Path::new("/work"),
        session_id,
        model: None,
        question: "why is it slow?",
    }
}

/// Forked, so the conversation's own session is left as it was; kept
/// nowhere, so nothing resumes it; with no tools, so it only answers.
#[test]
fn a_side_question_reads_the_conversation_and_writes_nothing() {
    let line = argv(&asked(Some("s1")));
    let at = line.iter().position(|a| a == "--resume").expect("resumes");
    assert_eq!(line[at + 1], "s1");
    for flag in ["--fork-session", "--no-session-persistence", "--print"] {
        assert!(line.contains(&flag.to_owned()), "{flag} missing: {line:?}");
    }
    let tools = line.iter().position(|a| a == "--tools").expect("tools");
    assert_eq!(line[tools + 1], "", "no tools at all");
    assert!(!argv(&asked(None))
        .iter()
        .any(|a| a == "--resume" || a == "--fork-session"));
}

/// Run against a stand-in for the CLI that answers with its own arguments.
#[cfg(unix)]
#[test]
fn a_side_question_is_answered_from_the_clis_result() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\nread -r question\n\
         printf '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Asked: %s\"}\\n' \"$question\"\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.to_string_lossy().into_owned();
    let aside = Aside {
        command: &command,
        cwd: dir.path(),
        ..asked(Some("s1"))
    };
    let said = (0..5)
        .find_map(|_| match ask(&aside) {
            Err(crate::AgentError::NotInstalled) => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                None
            }
            other => Some(other),
        })
        .expect("ran");
    assert_eq!(said.expect("an answer"), "Asked: why is it slow?");
}

#[cfg(unix)]
#[test]
fn a_side_question_the_cli_failed_says_why() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join("fake-claude");
    std::fs::write(
        &cli,
        "#!/bin/sh\nread -r _q\nprintf '{\"type\":\"result\",\"is_error\":true,\"result\":\"API Error: 529 Overloaded\"}\\n'\n",
    )
    .expect("script");
    std::fs::set_permissions(&cli, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let command = cli.to_string_lossy().into_owned();
    let aside = Aside {
        command: &command,
        cwd: dir.path(),
        ..asked(None)
    };
    let said = (0..5)
        .find_map(|_| match ask(&aside) {
            Err(crate::AgentError::NotInstalled) => {
                std::thread::sleep(std::time::Duration::from_millis(50));
                None
            }
            other => Some(other),
        })
        .expect("ran");
    let err = said.expect_err("a failure").to_string();
    assert!(err.contains("API Error: 529 Overloaded"), "{err}");
}
