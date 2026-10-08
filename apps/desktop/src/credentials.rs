//! Sign-ins a session will need, checked before it starts, and the ones it
//! turns out to lack while it runs.
//!
//! A session that finds its AWS profile expired halfway through a task stops
//! there, or keeps retrying, and the person learns of it from its last screen.
//! Before a start devpit checks what the project needs — an AWS profile named
//! in its `.env`, or what its `needs.txt` in devpit's folder for the project
//! lists — and says the command that fixes it. While it runs, a tool that
//! fails for want of a sign-in rings the bell with the same command.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// How long one check may take: a CLI that hangs is not a sign-in that failed.
const PATIENCE: Duration = Duration::from_secs(8);
/// The most of a `.env` file read when looking for a profile.
const ENV_CEILING: u64 = 64 * 1024;

/// A sign-in a session needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Need {
    Aws(String),
    Gcloud,
    Gh,
}

/// What is missing, and what fixes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Missing {
    pub what: String,
    pub command: String,
}

impl Need {
    fn missing(&self) -> Missing {
        match self {
            Need::Aws(profile) => Missing {
                what: format!("the AWS profile {profile} is signed out or expired"),
                command: format!("aws sso login --profile {profile}"),
            },
            Need::Gcloud => Missing {
                what: "gcloud is signed out or its token expired".to_owned(),
                command: "gcloud auth login".to_owned(),
            },
            Need::Gh => Missing {
                what: "the GitHub CLI is signed out".to_owned(),
                command: "gh auth login".to_owned(),
            },
        }
    }

    /// The command that answers whether the sign-in holds.
    fn probe(&self) -> (&'static str, Vec<String>) {
        match self {
            Need::Aws(profile) => (
                "aws",
                vec![
                    "sts".into(),
                    "get-caller-identity".into(),
                    "--profile".into(),
                    profile.clone(),
                ],
            ),
            Need::Gcloud => (
                "gcloud",
                vec!["auth".into(), "print-access-token".into(), "--quiet".into()],
            ),
            Need::Gh => ("gh", vec!["auth".into(), "status".into()]),
        }
    }
}

/// A profile name as AWS spells them; anything else is not passed to a command.
fn plain(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
}

/// The needs `needs.txt` declares: `aws <profile>`, `gcloud`, `gh`, one a line.
pub(crate) fn declared(text: &str) -> Vec<Need> {
    text.lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            match (words.next()?, words.next()) {
                ("aws", Some(profile)) if plain(profile) => Some(Need::Aws(profile.to_owned())),
                ("gcloud", None) => Some(Need::Gcloud),
                ("gh", None) => Some(Need::Gh),
                _ => None,
            }
        })
        .collect()
}

/// The AWS profile an env file names, if it names one.
pub(crate) fn aws_profile_in(env: &str) -> Option<String> {
    env.lines().find_map(|line| {
        let line = line.trim().trim_start_matches("export ").trim();
        let value = line.strip_prefix("AWS_PROFILE=")?;
        let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
        plain(value).then(|| value.to_owned())
    })
}

/// What a project's sessions need: declared, and the AWS profile its `.env`
/// files name.
pub(crate) fn needs_of(root: &Path, declared_file: Option<&Path>) -> Vec<Need> {
    let mut needs = declared_file
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|text| declared(&text))
        .unwrap_or_default();
    for name in [".env", ".env.local", ".envrc"] {
        let Ok(file) = std::fs::File::open(root.join(name)) else {
            continue;
        };
        let mut text = String::new();
        let _ =
            std::io::Read::read_to_string(&mut std::io::Read::take(file, ENV_CEILING), &mut text);
        if let Some(profile) = aws_profile_in(&text) {
            needs.push(Need::Aws(profile));
        }
    }
    needs.sort_by_key(|need| format!("{need:?}"));
    needs.dedup();
    needs
}

/// Whether the sign-in holds. A CLI that is not installed, or does not answer
/// in time, says nothing either way and counts as holding.
fn holds(need: &Need) -> bool {
    let (program, args) = need.probe();
    let Ok(mut child) = Command::new(program)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return true;
    };
    let began = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if began.elapsed() < PATIENCE => {
                std::thread::sleep(Duration::from_millis(100))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return true;
            }
        }
    }
}

/// What a project's sessions would start without, checked side by side.
pub(crate) fn before_start(root: &Path, declared_file: Option<&Path>) -> Vec<Missing> {
    let needs = needs_of(root, declared_file);
    let checks: Vec<_> = needs
        .into_iter()
        .map(|need| std::thread::spawn(move || (!holds(&need)).then(|| need.missing())))
        .collect();
    checks
        .into_iter()
        .filter_map(|check| check.join().ok().flatten())
        .collect()
}

/// The sign-in a failed tool's error says is missing, if it says.
pub(crate) fn missing_in(tool: &str, error: &str) -> Option<Missing> {
    let said = error.to_lowercase();
    let aws = [
        "expiredtoken",
        "the sso session associated with this profile has expired",
        "token has expired and refresh failed",
        "error loading sso token",
        "unable to locate credentials",
    ];
    if aws.iter().any(|sign| said.contains(sign)) {
        let profile = profile_named(error);
        return Some(match profile {
            Some(profile) => Need::Aws(profile).missing(),
            None => Missing {
                what: "AWS credentials are missing or expired".to_owned(),
                command: "aws sso login".to_owned(),
            },
        });
    }
    let gcloud = [
        "reauthentication failed",
        "there was a problem refreshing your current auth tokens",
        "you do not currently have an active account selected",
    ];
    if gcloud.iter().any(|sign| said.contains(sign)) {
        return Some(Need::Gcloud.missing());
    }
    if said.contains("gh auth login") || said.contains("you are not logged into any github hosts") {
        return Some(Need::Gh.missing());
    }
    let server = tool.strip_prefix("mcp__")?.split("__").next()?;
    let refused = [
        "401",
        "unauthorized",
        "invalid_token",
        "authentication required",
    ];
    refused
        .iter()
        .any(|sign| said.contains(sign))
        .then(|| Missing {
            what: format!("the MCP server {server} needs signing in again"),
            command: "/mcp".to_owned(),
        })
}

/// A `--profile <name>` an error repeats from the command that failed.
fn profile_named(error: &str) -> Option<String> {
    let mut words = error.split_whitespace();
    while let Some(word) = words.next() {
        if word == "--profile" {
            return words
                .next()
                .map(|name| name.trim_matches(|c| c == '"' || c == '\''))
                .filter(|name| plain(name))
                .map(str::to_owned);
        }
        if let Some(name) = word.strip_prefix("--profile=") {
            return plain(name).then(|| name.to_owned());
        }
    }
    None
}

/// Whether `command` is news for `session`: the same missing sign-in fails
/// every call after the first, and one bell is enough.
pub(crate) fn first_time(session: &str, command: &str) -> bool {
    static TOLD: std::sync::OnceLock<std::sync::Mutex<Vec<(String, String)>>> =
        std::sync::OnceLock::new();
    let Ok(mut told) = TOLD.get_or_init(Default::default).lock() else {
        return true;
    };
    let key = (session.to_owned(), command.to_owned());
    if told.contains(&key) {
        return false;
    }
    told.push(key);
    true
}

/// The sentence a refused start answers with.
pub(crate) fn refused(missing: &[Missing]) -> String {
    let each: Vec<String> = missing
        .iter()
        .map(|one| format!("{} — `{}`", one.what, one.command))
        .collect();
    format!(
        "not started: {}. Ask the person to run it, or start anyway with anyway: true once they say so.",
        each.join("; ")
    )
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
