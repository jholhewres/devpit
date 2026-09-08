//! The size ratchet, which only ever tightens.

use std::path::{Path, PathBuf};

use crate::Finding;

/// Where the ceilings live. One `path limit` pair per line.
pub(crate) const CEILINGS: &str = include_str!("../ceilings.txt");

/// Files may not grow past their ceiling.
///
/// A ceiling that can be raised is not a ceiling, so `cargo xtask ceilings`
/// refuses to raise one — that is where the ratchet lives now.
///
/// It used to live here too: a ceiling above its file also failed, which meant
/// a file shrinking by one line broke the build until someone regenerated the
/// list. That caught real splits and also cost a refactor over four lines, and
/// a guard billing that often is a guard on its way to being deleted. Ceilings
/// are set to the next step above the file instead, so ordinary edits pass and
/// a file that has genuinely outgrown its shape still fails.
pub fn files_only_get_shorter(root: &Path) -> Vec<Finding> {
    ceilings_in(CEILINGS, root)
}

/// The guard over a given ceiling list, so a test can exercise it without
/// editing the real one.
fn ceilings_in(list: &str, root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    for entry in list.lines() {
        let entry = entry.split('#').next().unwrap_or(entry).trim();
        if entry.is_empty() {
            continue;
        }
        let Some((relative, ceiling)) = entry.rsplit_once(char::is_whitespace) else {
            continue;
        };
        let relative = relative.trim();
        let Ok(ceiling) = ceiling.trim().parse::<usize>() else {
            continue;
        };

        let Ok(text) = std::fs::read_to_string(root.join(relative)) else {
            findings.push(Finding {
                file: PathBuf::from(relative),
                line: 1,
                what: "has a ceiling but no file — drop the entry".to_owned(),
            });
            continue;
        };

        let lines = text.lines().count();
        if lines > ceiling {
            findings.push(Finding {
                file: PathBuf::from(relative),
                line: lines,
                what: format!("{lines} lines, past its ceiling of {ceiling} — split it"),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree with one file of `lines` lines, to point a ceiling at.
    fn sized(lines: usize) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("crates/fake/src");
        std::fs::create_dir_all(&src).expect("create tree");
        std::fs::write(src.join("lib.rs"), "//\n".repeat(lines)).expect("write");
        dir
    }

    const ENTRY: &str = "crates/fake/src/lib.rs 100\n";

    #[test]
    fn the_guard_catches_a_file_that_grew() {
        let dir = sized(120);
        let findings = ceilings_in(ENTRY, dir.path());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].what.contains("past its ceiling"));
    }

    /// A file under its ceiling is a file with room to work in. The ratchet
    /// that stops the number climbing lives in `reseed`, not here.
    #[test]
    fn the_guard_is_quiet_about_a_file_with_room_left() {
        assert!(ceilings_in(ENTRY, sized(80).path()).is_empty());
        assert!(ceilings_in(ENTRY, sized(100).path()).is_empty());
    }

    /// A ceiling left behind by a deleted file is noise that never resolves.
    #[test]
    fn the_guard_reports_a_ceiling_with_no_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let findings = ceilings_in("gone.rs 10\n", dir.path());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].what.contains("no file"));
    }

    /// Comments and blank lines are not entries.
    #[test]
    fn the_guard_ignores_comments() {
        assert!(ceilings_in("# a note\n\n", Path::new("/nonexistent")).is_empty());
    }

    #[test]
    fn the_real_repository_passes() {
        let findings = files_only_get_shorter(&crate::workspace_root());
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
    }
}
