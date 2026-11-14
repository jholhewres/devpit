//! A side question (`/btw`): asked of a conversation's context, answered
//! apart from it, by a process of its own.

use std::path::Path;

use crate::AgentError;

/// What a side question needs.
pub struct Aside<'a> {
    pub command: &'a str,
    pub env: &'a [(String, String)],
    pub cwd: &'a Path,
    /// The conversation it reads, when it has started.
    pub session_id: Option<&'a str>,
    pub model: Option<&'a str>,
    pub question: &'a str,
}

/// The most of an answer read: a side question is answered in a few lines.
const MOST: u64 = 1024 * 1024;

/// Forked, so the conversation's session is left as it was; kept nowhere, so
/// its history never holds it; and with no tools, so it only answers.
pub(crate) fn argv(aside: &Aside<'_>) -> Vec<String> {
    let mut argv: Vec<String> = [
        "--print",
        "--output-format",
        "json",
        "--no-session-persistence",
        "--strict-mcp-config",
        "--tools",
        "",
    ]
    .map(str::to_owned)
    .to_vec();
    if let Some(session) = aside.session_id {
        argv.extend([
            "--resume".to_owned(),
            session.to_owned(),
            "--fork-session".to_owned(),
        ]);
    }
    if let Some(model) = aside.model {
        argv.extend(["--model".to_owned(), model.to_owned()]);
    }
    argv
}

/// The answer, or why there is none.
pub fn ask(aside: &Aside<'_>) -> Result<String, AgentError> {
    use std::io::{Read, Write};

    let mut child = crate::talk::own_group(&mut devpit_pty::host_env::command(aside.command))
        .args(argv(aside))
        .current_dir(aside.cwd)
        .envs(aside.env.iter().map(|(key, value)| (key, value)))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|_| AgentError::NotInstalled)?;
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{}", aside.question)
            .map_err(|err| AgentError::Unreadable(err.to_string()))?;
    }
    let mut said = String::new();
    if let Some(stdout) = child.stdout.take() {
        stdout
            .take(MOST)
            .read_to_string(&mut said)
            .map_err(|err| AgentError::Unreadable(err.to_string()))?;
    }
    let _ = child.wait();
    let result: serde_json::Value = serde_json::from_str(said.trim())
        .map_err(|_| AgentError::Unreadable("the side question had no answer".to_owned()))?;
    let answer = result["result"].as_str().unwrap_or_default().to_owned();
    if result["is_error"].as_bool().unwrap_or(false) {
        return Err(AgentError::Failed {
            command: "/btw".to_owned(),
            stderr: answer,
        });
    }
    Ok(answer)
}

#[cfg(test)]
#[path = "aside_tests.rs"]
mod tests;
