//! What a person's own agent command does, read by running it.
//!
//! `claudin` and `glm` are usually shell functions: a `claude` with a config
//! directory, an endpoint and a token in front of it. A function exists in an
//! interactive shell and nowhere else, so the chat and the board — which start
//! a program, not a shell — could not use one, and the person was asked to
//! copy its variables into a form by hand.
//!
//! So the function is run once, in their own interactive shell, with a stand-in
//! for every agent program at the front of `PATH`. The stand-in writes down
//! which program was called, with what arguments and in what environment, and
//! exits. What the function added to the environment, it added on purpose —
//! that difference, the program and the arguments are the profile.

use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use devpit_rpc::EnvVar;

/// The marker the probe passes, so the arguments the function added are told
/// apart from the ones it passed through.
const MARK: &str = "__devpit_probe__";

/// Long enough for a slow shell config; short enough that a function which
/// waits for something is abandoned rather than waited on.
const WAITS: Duration = Duration::from_secs(12);

/// The agents' own variables. Taken out of the shell the probe starts: one this
/// app inherited — devpit opened from inside a second account's session — would
/// otherwise equal what the function sets, and a variable set to what it
/// already was reads as one never set.
const AGENTS_OWN: &[&str] = &[
    "CLAUDE_",
    "ANTHROPIC_",
    "OPENAI_",
    "CODEX_",
    "GEMINI_",
    "GOOGLE_API_KEY",
];

/// Variables that differ between any two shells, whatever the function did.
const NOISE: &[&str] = &[
    "PWD", "OLDPWD", "SHLVL", "_", "PATH", "COLUMNS", "LINES", "RANDOM", "SECONDS",
];

/// What the command turned out to run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Probed {
    /// The agent program it called: `claude`, `codex`.
    pub program: String,
    /// The arguments it put in front of the ones it was given.
    pub args: Vec<String>,
    /// What it set that the shell had not.
    pub env: Vec<EnvVar>,
}

/// The stand-in: records its name, its arguments and its environment, then
/// exits without starting anything.
const STAND_IN: &str = r#"#!/bin/sh
d=$(dirname "$0")
basename "$0" > "$d/.which"
for a in "$@"; do printf '%s\0' "$a"; done > "$d/.argv"
env -0 > "$d/.env" 2>/dev/null || env > "$d/.env"
exit 0
"#;

/// Runs `name` in `shell` with stand-ins for `programs`, and reads what it did.
/// `None` when it called none of them — an absolute path, a script that does
/// something else — which is a command this cannot read.
pub(crate) fn probe(shell: &str, name: &str, programs: &[&str]) -> Result<Option<Probed>, String> {
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
        || name.is_empty()
    {
        return Err("only a command's name can be read, not a line".to_owned());
    }
    let dir = Scratch::new()?;
    for program in programs {
        let path = dir.path().join(program);
        std::fs::write(&path, STAND_IN).map_err(|err| err.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .map_err(|err| err.to_string())?;
        }
    }
    let at = dir.path().display().to_string().replace('\'', "'\\''");
    // The baseline is taken in the same shell, after the same config, just
    // before the function runs: only what the function adds differs.
    let script = format!(
        "PATH='{at}':\"$PATH\"; export PATH; (env -0 2>/dev/null || env) > '{at}/.base'; {name} {MARK} </dev/null >/dev/null 2>&1"
    );
    let mut command = devpit_pty::host_env::command(shell);
    for (name, _) in std::env::vars() {
        if AGENTS_OWN.iter().any(|prefix| name.starts_with(prefix)) {
            command.env_remove(&name);
        }
    }
    let mut child = command
        .args(["-ic", &script])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| err.to_string())?;
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() > WAITS => {
                let _ = child.kill();
                return Err(format!(
                    "{name} did not finish — it may be waiting for something"
                ));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(err) => return Err(err.to_string()),
        }
    }
    read(dir.path())
}

/// A folder of its own for the stand-ins, gone when the probe is.
struct Scratch(std::path::PathBuf);

impl Scratch {
    fn new() -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!("devpit-probe-{}", ulid::Ulid::generate()));
        std::fs::create_dir(&path).map_err(|err| err.to_string())?;
        Ok(Self(path))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn read(dir: &Path) -> Result<Option<Probed>, String> {
    let Ok(program) = std::fs::read_to_string(dir.join(".which")) else {
        return Ok(None);
    };
    let argv = std::fs::read(dir.join(".argv")).unwrap_or_default();
    let mut args: Vec<String> = argv
        .split(|byte| *byte == 0)
        .filter(|one| !one.is_empty())
        .map(|one| String::from_utf8_lossy(one).into_owned())
        .collect();
    if let Some(at) = args.iter().position(|one| one == MARK) {
        args.truncate(at);
    }
    let base = variables(&std::fs::read(dir.join(".base")).unwrap_or_default());
    let seen = variables(&std::fs::read(dir.join(".env")).unwrap_or_default());
    let env = seen
        .into_iter()
        .filter(|(name, value)| {
            !NOISE.contains(&name.as_str()) && base.iter().all(|(n, v)| n != name || v != value)
        })
        .map(|(name, value)| EnvVar { name, value })
        .collect();
    Ok(Some(Probed {
        program: program.trim().to_owned(),
        args,
        env,
    }))
}

/// `NAME=value` pairs, split on NUL when `env -0` was there and on newlines
/// when it was not.
fn variables(raw: &[u8]) -> Vec<(String, String)> {
    let split: Vec<&[u8]> = if raw.contains(&0) {
        raw.split(|byte| *byte == 0).collect()
    } else {
        raw.split(|byte| *byte == b'\n').collect()
    };
    split
        .into_iter()
        .filter_map(|one| {
            let text = String::from_utf8_lossy(one);
            let (name, value) = text.split_once('=')?;
            (!name.is_empty()).then(|| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bash whose "config" defines the function, standing in for their .bashrc.
    fn shell_with(function: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::tempdir().expect("tempdir");
        let rc = dir.path().join("rc");
        std::fs::write(&rc, function).expect("rc");
        let shell = dir.path().join("sh");
        std::fs::write(
            &shell,
            format!("#!/bin/sh\nexec bash --rcfile '{}' \"$@\"\n", rc.display()),
        )
        .expect("shell");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755))
                .expect("chmod");
        }
        let path = shell.display().to_string();
        (dir, path)
    }

    #[test]
    fn a_function_is_read_as_its_program_its_arguments_and_what_it_set() {
        let (_dir, shell) = shell_with(
            "claudin() { CLAUDE_CONFIG_DIR=\"$HOME/.claude-two\" ANTHROPIC_MODEL='glm-5.3[1m]' command claude --verbose \"$@\"; }\n",
        );
        let probed = probe(&shell, "claudin", &["claude", "codex"])
            .expect("ran")
            .expect("read");
        assert_eq!(probed.program, "claude");
        assert_eq!(probed.args, ["--verbose"]);
        let names: Vec<_> = probed.env.iter().map(|one| one.name.as_str()).collect();
        assert!(
            names.contains(&"CLAUDE_CONFIG_DIR") && names.contains(&"ANTHROPIC_MODEL"),
            "{names:?}"
        );
        assert!(
            !names.contains(&"PWD") && !names.contains(&"HOME"),
            "{names:?}"
        );
        let model = probed
            .env
            .iter()
            .find(|one| one.name == "ANTHROPIC_MODEL")
            .unwrap();
        assert_eq!(model.value, "glm-5.3[1m]");
    }

    #[test]
    fn what_the_function_sets_is_read_even_when_this_app_already_had_it() {
        // devpit started from inside that very account's session.
        std::env::set_var("CLAUDE_CONFIG_DIR", "/somewhere/.claude-two");
        let (_dir, shell) = shell_with(
            "two() { CLAUDE_CONFIG_DIR=/somewhere/.claude-two command claude \"$@\"; }\n",
        );
        let probed = probe(&shell, "two", &["claude"])
            .expect("ran")
            .expect("read");
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        assert!(
            probed.env.iter().any(|one| one.name == "CLAUDE_CONFIG_DIR"),
            "{:?}",
            probed.env
        );
    }

    #[test]
    fn a_command_that_calls_no_agent_is_not_read() {
        let (_dir, shell) = shell_with("other() { true; }\n");
        assert_eq!(probe(&shell, "other", &["claude"]).expect("ran"), None);
    }

    #[test]
    fn only_a_name_is_run() {
        assert!(probe("/bin/sh", "rm -rf /", &["claude"]).is_err());
    }
}
