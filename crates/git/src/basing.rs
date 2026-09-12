//! Where a card's worktree is allowed to land.
//!
//! The folder used to be one constant — `~/.devpit/worktrees` — and a constant
//! needs no rules. A path somebody types into a settings field does: it is
//! handed to `git worktree add`, so what it may not be is the whole of this
//! module.
//!
//! Relative means "inside this project", absolute means "one shared folder".
//! They differ in more than resolution: a relative base is already inside the
//! project, so it carries no project segment, while an absolute one holds
//! every project at once and would collide the moment two of them had a card
//! with the same id.

use std::path::{Component, Path, PathBuf};

/// Why a base was refused, in words the field can print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// A segment that climbs. A relative base with `..` leaves the project,
    /// which is the one thing "per-project location" promises it will not do.
    Climbs,
    /// Inside `.git`. A checkout in the object store is a repository inside
    /// its own database, and git will happily start making one.
    InsideGit,
    /// The project root itself, or a path that resolves to it. The main
    /// checkout is how an agent ends up committing to the branch the person is
    /// sitting on.
    TheProjectItself,
}

impl std::fmt::Display for Refused {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let said = match self {
            Self::Climbs => "a worktree folder cannot climb out of the project with `..`",
            Self::InsideGit => "a worktree cannot live inside `.git`",
            Self::TheProjectItself => "that is the project itself, not a place to put a worktree",
        };
        out.write_str(said)
    }
}

/// Reads what was typed. Empty and whitespace are both "use the default".
pub fn chosen(typed: &str) -> Option<&str> {
    let trimmed = typed.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

/// Whether a base may be used at all, before any card names a folder under it.
///
/// Checked on the way in rather than on the way out: a base saved today is
/// used by every worktree made from here on, and finding out it was wrong at
/// `git worktree add` time means finding out once per card.
pub fn allowed(project_root: &Path, typed: &str) -> Result<(), Refused> {
    let Some(base) = chosen(typed) else {
        return Ok(());
    };
    let path = Path::new(base);

    if path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err(Refused::Climbs);
    }

    let resolved = resolve(project_root, base);
    if resolved.components().any(|part| part.as_os_str() == ".git") {
        return Err(Refused::InsideGit);
    }

    // Through the filesystem, not through the strings: a symlink pointing at
    // the project root compares equal to nothing and resolves to everything.
    let root = std::fs::canonicalize(project_root).unwrap_or_else(|_| project_root.to_path_buf());
    let here = std::fs::canonicalize(&resolved).unwrap_or(resolved);
    if here == root {
        return Err(Refused::TheProjectItself);
    }

    Ok(())
}

/// The folder the base names, with nothing under it yet.
fn resolve(project_root: &Path, base: &str) -> PathBuf {
    let path = Path::new(base);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    project_root.join(path)
}

/// Where this card's worktree goes.
///
/// `home` is the devpit workspace, used when nothing was chosen. The project
/// segment appears only for an absolute base: a relative one is already inside
/// the project it belongs to, and repeating the id there would bury every
/// checkout one level deeper for nothing.
pub fn worktree_at(
    typed: &str,
    home: &Path,
    project_root: &Path,
    project_id: &str,
    card_id: &str,
) -> PathBuf {
    let Some(base) = chosen(typed) else {
        return crate::worktree_home(home, project_id, card_id);
    };
    let root = resolve(project_root, base);
    if Path::new(base).is_absolute() {
        root.join(project_id).join(card_id)
    } else {
        root.join(card_id)
    }
}

#[cfg(test)]
#[path = "basing_tests.rs"]
mod tests;
