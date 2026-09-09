//! `git status --porcelain=v2` and `git diff --numstat`, parsed.
//!
//! Version 2 of the porcelain format rather than v1: v1 cannot report how far
//! the branch has drifted from its upstream, and it reports a rename in a way
//! that has to be guessed at. v2 states both.

use std::collections::BTreeMap;
use std::path::Path;

use devpit_rpc::{Change, GitStatus};

use crate::{run, GitError};

/// What the chrome around a checkout reports.
#[derive(Debug, Clone, Default)]
pub struct Status {
    /// Branch name, or a short oid when HEAD is detached.
    pub branch: String,
    pub ahead: u32,
    pub behind: u32,
    /// Path relative to the worktree root → how it stands.
    pub paths: BTreeMap<String, GitStatus>,
}

impl Status {
    pub fn dirty_files(&self) -> u32 {
        self.paths.len() as u32
    }
}

pub fn status(root: &Path) -> Result<Status, GitError> {
    // `-z` because a path may contain a newline. It is rare and it is legal,
    // and the line-oriented form has no way to tell that apart from a second
    // entry — which is the kind of bug that only ever appears on someone
    // else's machine.
    let raw = run(
        root,
        &[
            "status",
            "--porcelain=v2",
            "--branch",
            "--untracked-files=all",
            "-z",
        ],
    )?;
    Ok(parse_status(&raw))
}

fn parse_status(raw: &str) -> Status {
    let mut status = Status::default();
    let mut records = raw.split('\0').filter(|record| !record.is_empty());

    while let Some(record) = records.next() {
        let mut parts = record.splitn(2, ' ');
        let (kind, rest) = (
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
        );

        match kind {
            "#" => read_header(&mut status, rest),
            // `1 <xy> <sub> <mH> <mI> <mW> <hH> <hI> <path>` — seven fields
            // before the path once the record kind is off, and the path is
            // everything after them.
            "1" => {
                if let Some((code, path)) = split_after(rest, 7) {
                    status.paths.insert(path.to_owned(), from_code(code));
                }
            }
            // `2 <xy> … <X><score> <path>` and then the original path as its
            // own record. A rename is a change to the new path; the old one is
            // consumed so it is not read as a second entry.
            "2" => {
                if let Some((code, path)) = split_after(rest, 8) {
                    status.paths.insert(path.to_owned(), from_code(code));
                }
                records.next();
            }
            "?" => {
                status.paths.insert(rest.to_owned(), GitStatus::Untracked);
            }
            // `u` (unmerged) is deliberately absent: a conflict is not a
            // status this screen draws, and drawing it as "modified" would say
            // something untrue about work that needs a person.
            _ => {}
        }
    }

    status
}

fn read_header(status: &mut Status, rest: &str) {
    let mut parts = rest.split(' ');
    match parts.next() {
        Some("branch.head") => {
            let head = parts.next().unwrap_or_default();
            // Git writes the literal string `(detached)` here, which is not a
            // branch anyone can check out. The short oid is filled in by the
            // caller, which is the one that can ask for it.
            status.branch = head.to_owned();
        }
        Some("branch.ab") => {
            // `+2 -6`, always both, always signed.
            for token in parts {
                let (sign, digits) = token.split_at(1);
                let value = digits.parse().unwrap_or(0);
                match sign {
                    "+" => status.ahead = value,
                    "-" => status.behind = value,
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

/// Splits off `count` space-separated fields and returns the first field plus
/// the untouched remainder, which is the path.
///
/// The path is never split on, because a path may contain spaces.
fn split_after(rest: &str, count: usize) -> Option<(&str, &str)> {
    let mut offset = 0;
    let mut first = "";
    for index in 0..count {
        let slice = &rest[offset..];
        let end = slice.find(' ')?;
        if index == 0 {
            first = &slice[..end];
        }
        offset += end + 1;
    }
    Some((first, rest.get(offset..)?))
}

/// The two-letter code is `<index><worktree>`.
///
/// The worktree column wins when it says anything, because it describes the
/// file as it is on disk — which is the file the person is looking at.
fn from_code(code: &str) -> GitStatus {
    let mut chars = code.chars();
    let index = chars.next().unwrap_or('.');
    let worktree = chars.next().unwrap_or('.');

    match (index, worktree) {
        (_, 'D') | ('D', '.') => GitStatus::Deleted,
        (_, 'M') | (_, 'T') => GitStatus::Modified,
        ('A', _) => GitStatus::Added,
        ('M' | 'R' | 'C', _) => GitStatus::Modified,
        _ => GitStatus::Modified,
    }
}

/// Every changed path with the size of its edit.
///
/// Two calls, because git has no single one that answers both: `--numstat`
/// counts lines for tracked files and says nothing about untracked ones, and
/// `status` knows about untracked files and counts nothing.
pub fn changes(root: &Path) -> Result<Vec<Change>, GitError> {
    let status = status(root)?;
    let mut counted = BTreeMap::new();

    // `HEAD` fails on a repository with no commits, which is a normal state
    // for a project someone just created. There is nothing to diff against
    // there, and every path is simply untracked.
    if let Ok(raw) = run(root, &["diff", "--numstat", "-z", "HEAD"]) {
        for record in raw.split('\0').filter(|record| !record.is_empty()) {
            let mut fields = record.splitn(3, '\t');
            let added = fields.next().and_then(|f| f.parse().ok());
            let removed = fields.next().and_then(|f| f.parse().ok());
            let path = fields.next().unwrap_or_default();
            // A binary file is reported as `-\t-\t<path>`; it is a real change
            // with no line count, and zero is the truthful answer for it.
            counted.insert(path.to_owned(), (added.unwrap_or(0), removed.unwrap_or(0)));
        }
    }

    Ok(status
        .paths
        .into_iter()
        .map(|(path, status)| {
            let (added, removed) = counted.get(&path).copied().unwrap_or((0, 0));
            Change {
                path,
                status,
                added,
                removed,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn a_path_with_spaces_survives_the_split() {
        // The field count is what protects this. Splitting on whitespace and
        // taking the last token would turn `my notes.md` into `notes.md`.
        let record = "1 .M N... 100644 100644 100644 aaa bbb my notes.md";
        let (code, path) = split_after(record.strip_prefix("1 ").unwrap(), 7).expect("split");
        assert_eq!(code, ".M");
        assert_eq!(path, "my notes.md");
    }

    #[test]
    fn drift_is_read_from_the_branch_header() {
        let raw = "# branch.head jholhewres/visual\0# branch.ab +2 -6\0";
        let status = parse_status(raw);
        assert_eq!(status.branch, "jholhewres/visual");
        assert_eq!(status.ahead, 2);
        assert_eq!(status.behind, 6);
    }

    #[test]
    fn a_rename_counts_once_and_swallows_its_old_path() {
        // Without consuming the trailing record the old path is read as a
        // second entry, and the dirty count comes out one too high.
        let raw = "2 R. N... 100644 100644 100644 aaa bbb R100 new.rs\0old.rs\0";
        let status = parse_status(raw);
        assert_eq!(status.paths.len(), 1, "the old path was counted too");
        assert!(status.paths.contains_key("new.rs"));
    }

    #[test]
    fn reads_a_real_repository() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");

        std::fs::write(dir.path().join("a.txt"), "one\ntwo\n").expect("write");
        std::fs::write(dir.path().join("b.txt"), "new\n").expect("write");

        let status = status(dir.path()).expect("status");
        assert_eq!(status.branch, "main");
        assert_eq!(status.dirty_files(), 2);
        assert_eq!(status.paths.get("a.txt"), Some(&GitStatus::Modified));
        assert_eq!(status.paths.get("b.txt"), Some(&GitStatus::Untracked));

        let changes = changes(dir.path()).expect("changes");
        let modified = changes.iter().find(|c| c.path == "a.txt").expect("a.txt");
        assert_eq!((modified.added, modified.removed), (1, 0));

        // The untracked file has no line count, and says so as zero rather
        // than being left out of the list.
        let untracked = changes.iter().find(|c| c.path == "b.txt").expect("b.txt");
        assert_eq!(untracked.status, GitStatus::Untracked);
    }
}
