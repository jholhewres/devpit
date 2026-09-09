//! `git worktree list --porcelain`, and the status of each checkout.

use std::path::{Path, PathBuf};

use devpit_rpc::Worktree;

use crate::{identify, run, status::status, GitError};

/// Every checkout of the repository, current one first.
///
/// Each carries its own branch and dirt, read from that checkout rather than
/// from the one the caller happened to name: two worktrees of the same
/// repository disagree about both, and that disagreement is the reason the
/// list exists.
pub fn worktrees(root: &Path) -> Result<Vec<Worktree>, GitError> {
    let raw = run(root, &["worktree", "list", "--porcelain"])?;
    let current = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    let mut found = Vec::new();
    for block in raw.split("\n\n") {
        let Some(path) = field(block, "worktree ").map(PathBuf::from) else {
            continue;
        };

        // A checkout listed by git but missing on disk is a stale entry from a
        // deleted folder. Skipped rather than shown: a row that cannot be
        // opened is a row that only invites a click that fails.
        if !path.is_dir() {
            continue;
        }

        let here = std::fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
        let read = status(&path);

        found.push(Worktree {
            id: identify(&here),
            branch: branch_of(block, &read),
            folder: here
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| here.display().to_string()),
            ahead: read.as_ref().map(|s| s.ahead).unwrap_or(0),
            behind: read.as_ref().map(|s| s.behind).unwrap_or(0),
            // Null, not zero: a checkout whose git could not be read has an
            // unknown amount of work in it, and zero would claim otherwise.
            dirty_files: read.as_ref().ok().map(|s| s.dirty_files()),
            current: here == current,
        });
    }

    // The one you are in comes first; the rest keep git's own order, which is
    // creation order and stable across calls.
    found.sort_by_key(|worktree| !worktree.current);
    Ok(found)
}

/// The branch, or something a person can act on when there is not one.
fn branch_of(block: &str, read: &Result<crate::Status, GitError>) -> String {
    if let Some(reference) = field(block, "branch ") {
        return reference
            .strip_prefix("refs/heads/")
            .unwrap_or(reference)
            .to_owned();
    }
    // Detached HEAD. `git status` writes the literal `(detached)` there, which
    // names no commit; the short oid does, and is what gets pasted into a
    // command.
    if let Some(head) = field(block, "HEAD ") {
        return head.chars().take(7).collect();
    }
    read.as_ref()
        .map(|status| status.branch.clone())
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// The folder a worktree id names.
///
/// The contract carries the folder's *name*, never its path — an absolute path
/// is not the screen's business and would end up in a React key. Turning the
/// id back into a path is this side's job, and it is done by asking git rather
/// than by keeping a map that can go stale.
pub fn worktree_path(root: &Path, id: &str) -> Result<Option<PathBuf>, GitError> {
    let raw = run(root, &["worktree", "list", "--porcelain"])?;

    Ok(raw
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(|path| PathBuf::from(path.trim()))
        .find(|path| {
            let here = std::fs::canonicalize(path).unwrap_or_else(|_| path.clone());
            identify(&here) == id
        }))
}

fn field<'a>(block: &'a str, prefix: &str) -> Option<&'a str> {
    block
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .map(str::trim)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn an_id_round_trips_back_to_its_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");

        let found = worktrees(dir.path()).expect("worktrees");
        let path = worktree_path(dir.path(), &found[0].id)
            .expect("lookup")
            .expect("a path for the id it just handed out");

        let canonical = std::fs::canonicalize(&path).expect("canonicalize");
        assert_eq!(canonical, std::fs::canonicalize(dir.path()).expect("root"));
        assert!(worktree_path(dir.path(), "wt_nothing")
            .expect("lookup")
            .is_none());
    }

    #[test]
    fn lists_the_checkout_it_was_given_and_marks_it_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");

        let found = worktrees(dir.path()).expect("worktrees");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].branch, "main");
        assert!(found[0].current, "the checkout asked about is not marked");
        assert_eq!(found[0].dirty_files, Some(0));
    }

    #[test]
    fn a_second_checkout_reports_its_own_branch_and_dirt() {
        let dir = tempfile::tempdir().expect("tempdir");
        let main = dir.path().join("main");
        std::fs::create_dir_all(&main).expect("create");
        fixture::repo(&main);
        std::fs::write(main.join("a.txt"), "one\n").expect("write");
        fixture::commit(&main, "first");

        let side = dir.path().join("side");
        let ok = std::process::Command::new("git")
            .arg("-C")
            .arg(&main)
            .args(["worktree", "add", "-q", "-b", "side"])
            .arg(&side)
            .output()
            .expect("run git")
            .status
            .success();
        assert!(ok, "could not add a second worktree");

        // Dirty in the second checkout only. Reading the branch from the
        // caller's checkout would report `main` for both rows, and reading the
        // dirt from it would report zero for a tree that has work in it.
        std::fs::write(side.join("b.txt"), "two\n").expect("write");

        let found = worktrees(&main).expect("worktrees");
        assert_eq!(found.len(), 2);
        assert!(found[0].current, "the caller's checkout is not first");

        let other = found.iter().find(|w| w.branch == "side").expect("side");
        assert_eq!(other.dirty_files, Some(1), "dirt read from the wrong tree");
        assert!(!other.current);
    }
}
