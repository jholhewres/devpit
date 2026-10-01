//! A repository at a glance, for an orchestrator that would otherwise run
//! `git status`, `git log` and `git ls-remote` by hand to answer "where is
//! that work".

use std::path::Path;

use crate::invoke::run;
use crate::GitError;

/// One branch asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchState {
    pub name: String,
    pub local: bool,
    /// Known on `origin`, as far as the last fetch knows.
    pub remote: bool,
    /// Whether its tip is already in the default branch.
    pub merged: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoState {
    pub branch: String,
    /// Files changed and not committed.
    pub dirty: usize,
    pub upstream: Option<String>,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    /// The latest commits, `short sha` and subject.
    pub commits: Vec<(String, String)>,
    pub default_branch: Option<String>,
    pub branches: Vec<BranchState>,
    pub tag: Option<String>,
    pub since_tag: Option<u32>,
}

/// How many commits are listed.
const COMMITS: &str = "-5";

/// A branch name an agent may ask about: plain, and never an option.
pub fn plain_branch(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.contains("..")
        && name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "/._-".contains(ch))
}

fn count(root: &Path, args: &[&str]) -> Option<u32> {
    run(root, args).ok()?.trim().parse().ok()
}

/// Where the repository at `root` stands. With `fetch`, `origin` is asked
/// first; without, only what is already known locally is read.
pub fn repo_state(root: &Path, branches: &[String], fetch: bool) -> Result<RepoState, GitError> {
    if fetch {
        let _ = run(root, &["fetch", "--quiet", "--prune", "origin"]);
    }
    let branch = run(root, &["rev-parse", "--abbrev-ref", "HEAD"])?
        .trim()
        .to_owned();
    let dirty = run(root, &["status", "--porcelain"])?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    let upstream = run(
        root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .ok()
    .map(|said| said.trim().to_owned())
    .filter(|said| !said.is_empty());
    let (behind, ahead) = match run(
        root,
        &["rev-list", "--left-right", "--count", "@{u}...HEAD"],
    ) {
        Ok(said) => {
            let mut parts = said.split_whitespace().map(|part| part.parse::<u32>().ok());
            (parts.next().flatten(), parts.next().flatten())
        }
        Err(_) => (None, None),
    };
    let commits = run(root, &["log", COMMITS, "--format=%h%x09%s"])
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(sha, subject)| (sha.to_owned(), subject.to_owned()))
        .collect();
    let default_branch = run(
        root,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .ok()
    .map(|said| said.trim().to_owned())
    .filter(|said| !said.is_empty());
    let branches = branches
        .iter()
        .filter(|name| plain_branch(name))
        .map(|name| {
            let local = run(
                root,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("refs/heads/{name}"),
                ],
            )
            .is_ok();
            let remote = run(
                root,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("refs/remotes/origin/{name}"),
                ],
            )
            .is_ok();
            let tip = if local {
                name.clone()
            } else {
                format!("origin/{name}")
            };
            let merged = if local || remote {
                default_branch
                    .as_deref()
                    .map(|base| run(root, &["merge-base", "--is-ancestor", &tip, base]).is_ok())
            } else {
                None
            };
            BranchState {
                name: name.clone(),
                local,
                remote,
                merged,
            }
        })
        .collect();
    let tag = run(root, &["describe", "--tags", "--abbrev=0"])
        .ok()
        .map(|said| said.trim().to_owned())
        .filter(|said| !said.is_empty());
    let since_tag = tag
        .as_deref()
        .and_then(|tag| count(root, &["rev-list", "--count", &format!("{tag}..HEAD")]));
    Ok(RepoState {
        branch,
        dirty,
        upstream,
        ahead,
        behind,
        commits,
        default_branch,
        branches,
        tag,
        since_tag,
    })
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
