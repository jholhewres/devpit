//! Making a new worktree into a checkout that actually works.
//!
//! `node_modules/`, `target/`, `.env`, `vendor/` — none of it is versioned, so
//! none of it exists in a worktree that was just created. The first `make
//! test` in there fails, and it fails looking like the agent's fault. This is
//! the answer: a declared, visible preparation step.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::prime_paths::{copied, plain};

/// What a project needs done to a fresh checkout.
///
/// Lives in the devpit workspace rather than in the repository: it is one
/// person's local setup — an `.env` path, a shared cache — and committing it
/// would put it in everyone else's tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Prime {
    /// Files to copy from the main checkout: `.env*`, `config/*.local.json`.
    /// A `*` matches within one name; the folder part is written out.
    pub copy: Vec<String>,
    /// Files to link back to the main checkout instead of copying: one `.env`,
    /// read by every worktree.
    pub link: Vec<String>,
    /// Environment variables every command in the worktree gets. This is how a
    /// shared `CARGO_TARGET_DIR` is done — a symlink would have two builds
    /// writing the same directory.
    pub share: std::collections::BTreeMap<String, String>,
    /// Commands to run once, in the worktree.
    pub run: Vec<String>,
    /// Seconds each command may take before it is stopped and the preparation
    /// fails. Ten minutes unless said.
    pub timeout: Option<u64>,
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
///
/// Inside the worktree's own git folder, not beside its files: there it was
/// an untracked file, counted as unsaved work that refused an archive.
fn done_marker(at: &Path) -> PathBuf {
    let dot_git = at.join(".git");
    let git_dir = std::fs::read_to_string(&dot_git)
        .ok()
        .and_then(|text| {
            text.trim()
                .strip_prefix("gitdir:")
                .map(|dir| PathBuf::from(dir.trim()))
        })
        .map(|dir| if dir.is_absolute() { dir } else { at.join(dir) })
        .unwrap_or(dot_git);
    if git_dir.is_dir() {
        git_dir.join("devpit-primed")
    } else {
        at.join(".devpit-primed")
    }
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
    if done_marker(at).exists() || at.join(".devpit-primed").exists() {
        return Ok(Primed::Nothing);
    }
    if prime.copy.is_empty()
        && prime.link.is_empty()
        && prime.run.is_empty()
        && prime.share.is_empty()
    {
        return Ok(Primed::Nothing);
    }

    for relative in prime.copy.iter().flat_map(|pattern| copied(main, pattern)) {
        let to = at.join(&relative);
        if to.symlink_metadata().is_ok() {
            continue;
        }
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(main.join(&relative), &to)?;
        on_line(&format!("copied {relative}"));
    }

    for relative in prime.link.iter().filter(|one| plain(one)) {
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
        let log = done_marker(at).with_file_name("devpit-prime.log");
        if let Some(code) =
            crate::prime_command::ran(line, at, &prime.share, prime.timeout, &log, &mut on_line)?
        {
            return Ok(Primed::Failed {
                command: line.clone(),
                code,
            });
        }
    }

    std::fs::write(done_marker(at), "")?;
    Ok(Primed::Done)
}

#[cfg(test)]
#[path = "prime_tests.rs"]
mod tests;
