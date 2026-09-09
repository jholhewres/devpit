//! Creating and removing a card's worktree.
//!
//! `worktrees.rs` reads; this writes. Kept apart because the reading side is
//! called on every render and the writing side is called by a person.

use std::path::{Path, PathBuf};

use crate::{run, GitError};

/// A worktree that now exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Made {
    pub path: PathBuf,
    pub branch: String,
    /// The commit it started from, as a SHA.
    ///
    /// A SHA and not a branch name: with `main` as the base, the card's diff
    /// would change every time `main` moved, without the card changing. A card
    /// that changes on its own is a card nobody can trust.
    pub base_ref: String,
}

/// What removing a worktree would throw away.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Loss {
    pub files: usize,
    pub lines: usize,
}

impl Loss {
    pub fn is_empty(&self) -> bool {
        self.files == 0
    }
}

/// The branch a card's worktree gets.
///
/// The `devpit/` prefix does two jobs: it shows whose branch this is in a bare
/// `git branch`, and it makes the set safe to delete in bulk without catching
/// something a person made by hand.
pub fn branch_for(title: &str, card_id: &str) -> String {
    format!("devpit/{}-{}", slug(title), short(card_id))
}

fn slug(title: &str) -> String {
    let mut out = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    // Cut on a character boundary, and only after: a branch name is a path
    // component, and a long title is not worth a long one.
    let short: String = trimmed.chars().take(40).collect();
    let short = short.trim_end_matches('-');
    if short.is_empty() {
        // A title with nothing nameable in it still needs a branch.
        "card".to_owned()
    } else {
        short.to_owned()
    }
}

/// The tail of the id, which is the part that differs between two cards made
/// in the same second.
fn short(card_id: &str) -> String {
    let kept: String = card_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect();
    let from = kept.len().saturating_sub(8);
    kept[from..].to_lowercase()
}

/// Where a card's worktree lives.
///
/// Outside the repository on purpose: inside, it would show up in the file
/// tree, in `ripgrep`, in every watcher, and one day in a commit.
pub fn worktree_home(home: &Path, project_id: &str, card_id: &str) -> PathBuf {
    home.join("worktrees").join(project_id).join(card_id)
}

/// Whether a path may be handed to a card as its worktree.
///
/// The main checkout may not. That is how an agent ends up committing to the
/// branch the person is sitting on, without anyone having asked for it.
pub fn assignable(main: &Path, candidate: &Path) -> bool {
    let main = std::fs::canonicalize(main).unwrap_or_else(|_| main.to_path_buf());
    let candidate = std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf());
    candidate != main
}

/// Creates a worktree for a card, on a branch of its own.
///
/// `base` is resolved to a SHA here and now — never fetched. What is local is
/// what the card starts from, and the screen says how old that is.
pub fn create(root: &Path, at: &Path, branch: &str, base: &str) -> Result<Made, GitError> {
    if !assignable(root, at) {
        return Err(GitError::Refused(
            "the main checkout cannot be a card's worktree".to_owned(),
        ));
    }
    if at.exists() {
        return Err(GitError::Refused(format!(
            "{} already exists",
            at.display()
        )));
    }
    let base_ref = run(root, &["rev-parse", base])?.trim().to_owned();
    if let Some(parent) = at.parent() {
        std::fs::create_dir_all(parent).map_err(|err| GitError::Refused(err.to_string()))?;
    }
    run(
        root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            branch,
            &at.to_string_lossy(),
            &base_ref,
        ],
    )?;
    Ok(Made {
        path: at.to_path_buf(),
        branch: branch.to_owned(),
        base_ref,
    })
}

/// What is in a checkout that no commit holds.
pub fn uncommitted(path: &Path) -> Result<Loss, GitError> {
    let files = run(path, &["status", "--porcelain"])?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count();
    // Tracked changes only: an untracked file has no "before" to diff against,
    // and it is already counted as a file.
    let lines = run(path, &["diff", "--numstat", "HEAD"])?
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let added: usize = parts.next()?.parse().ok()?;
            let removed: usize = parts.next()?.parse().ok()?;
            Some(added + removed)
        })
        .sum();
    Ok(Loss { files, lines })
}

/// Removes a card's worktree.
///
/// The branch stays. The folder is disposable; what was committed in it is
/// not, and deleting both would make "remove" mean two different things.
pub fn remove(root: &Path, path: &Path, even_dirty: bool) -> Result<Loss, GitError> {
    let loss = uncommitted(path).unwrap_or_default();
    if !even_dirty && !loss.is_empty() {
        return Err(GitError::Refused(format!(
            "{} file(s) and {} line(s) are not committed",
            loss.files, loss.lines
        )));
    }
    let mut argv = vec!["worktree", "remove"];
    if even_dirty {
        argv.push("--force");
    }
    let target = path.to_string_lossy().into_owned();
    argv.push(&target);
    run(root, &argv)?;
    Ok(loss)
}

/// How much disk a folder takes, in bytes.
///
/// Shown because a feature that quietly eats forty gigabytes is a feature
/// people uninstall.
pub fn disk_usage(path: &Path) -> u64 {
    fn walk(dir: &Path, depth: usize) -> u64 {
        if depth > 32 {
            return 0;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return 0;
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| match entry.file_type() {
                Ok(kind) if kind.is_dir() => walk(&entry.path(), depth + 1),
                Ok(kind) if kind.is_file() => entry.metadata().map(|meta| meta.len()).unwrap_or(0),
                _ => 0,
            })
            .sum()
    }
    walk(path, 0)
}

/// Worktree folders under a project's home that no card claims any more.
///
/// The folder's name is the card id, so the comparison is a set difference
/// and not a guess.
pub fn orphans(home: &Path, project_id: &str, cards: &[String]) -> Vec<PathBuf> {
    let dir = home.join("worktrees").join(project_id);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false))
        .filter(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            !cards.contains(&name)
        })
        .map(|entry| entry.path())
        .collect()
}
