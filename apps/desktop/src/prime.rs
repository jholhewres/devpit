//! Making a new worktree into a checkout that actually works.
//!
//! `node_modules/`, `target/`, `.env`, `vendor/` — none of it is versioned, so
//! none of it exists in a worktree that was just created. The first `make
//! test` in there fails, and it fails looking like the agent's fault. This is
//! the answer: a declared, visible preparation step.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What a project needs done to a fresh checkout.
///
/// Lives in the devpit workspace rather than in the repository: it is one
/// person's local setup — an `.env` path, a shared cache — and committing it
/// would put it in everyone else's tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prime {
    /// Files to link back to the main checkout instead of copying: one `.env`,
    /// read by every worktree.
    pub link: Vec<String>,
    /// Environment variables every command in the worktree gets. This is how a
    /// shared `CARGO_TARGET_DIR` is done — a symlink would have two builds
    /// writing the same directory.
    pub share: std::collections::BTreeMap<String, String>,
    /// Commands to run once, in the worktree.
    pub run: Vec<String>,
}

/// How a preparation ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Primed {
    /// Nothing was declared, or it had already been done.
    Nothing,
    Done,
    /// Named as a failure *of the preparation*, with the command and its code,
    /// so it is never read as the work failing.
    Failed {
        command: String,
        code: i32,
    },
}

pub fn read(path: &Path) -> Prime {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// The marker that says this worktree has already been prepared.
fn done_marker(at: &Path) -> PathBuf {
    at.join(".devpit-primed")
}

/// Runs the preparation in a fresh worktree, once.
///
/// `on_line` gets the output as it happens: forty seconds of silence reads as
/// a hang, and the person is owed the difference.
pub fn run(
    prime: &Prime,
    main: &Path,
    at: &Path,
    mut on_line: impl FnMut(&str),
) -> std::io::Result<Primed> {
    if done_marker(at).exists() {
        return Ok(Primed::Nothing);
    }
    if prime.link.is_empty() && prime.run.is_empty() && prime.share.is_empty() {
        return Ok(Primed::Nothing);
    }

    for relative in &prime.link {
        let from = main.join(relative);
        let to = at.join(relative);
        if !from.exists() || to.exists() {
            continue;
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(&from, &to)?;
        on_line(&format!("linked {relative}"));
    }

    for line in &prime.run {
        on_line(&format!("$ {line}"));
        let output = devpit_pty::host_env::command("sh")
            .arg("-c")
            .arg(line)
            .current_dir(at)
            .envs(&prime.share)
            .output()?;
        on_line(&String::from_utf8_lossy(&output.stdout));
        if !output.status.success() {
            on_line(&String::from_utf8_lossy(&output.stderr));
            return Ok(Primed::Failed {
                command: line.clone(),
                code: output.status.code().unwrap_or(-1),
            });
        }
    }

    std::fs::write(done_marker(at), "")?;
    Ok(Primed::Done)
}

#[cfg(test)]
#[path = "prime_tests.rs"]
mod tests;
