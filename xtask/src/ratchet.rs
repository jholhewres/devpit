//! The size ratchet, which only ever tightens.

use std::path::{Path, PathBuf};

use crate::Finding;

/// Where the ceilings live. One `path limit` pair per line.
const CEILINGS: &str = include_str!("../ceilings.txt");

/// Files may not grow past their ceiling, and a ceiling may not sit above the
/// file it caps.
///
/// A ceiling that can be raised is not a ceiling: the file grows, someone bumps
/// the number, and the ratchet meant to stop the drift becomes the record of
/// it. Both halves are enforced here, so the only way to change a number is
/// down.
pub fn files_only_get_shorter(root: &Path) -> Vec<Finding> {
    ceilings_in(CEILINGS, root)
}

/// Files at or above this many lines get a ceiling. Below it, the ratchet
/// would be noise: short files churn, and every churn would be a failure.
const WORTH_CAPPING: usize = 120;

/// Rewrites `xtask/ceilings.txt` from what the tree measures now.
///
/// The guard fails when a ceiling sits above its file, which is correct and
/// also relentless — every time a file gets shorter, a number has to follow it
/// down. Doing that by hand across the whole list is how a guard earns enough
/// resentment to be deleted, so `cargo xtask ceilings` does it.
///
/// It can only tighten in practice: a file that grew fails `check` before
/// anyone gets to run this.
pub fn reseed(root: &Path) -> std::io::Result<usize> {
    let mut lines = vec![
        "# Size ceilings. One \"path limit\" per line.".to_owned(),
        "#".to_owned(),
        "# The ratchet only tightens: a file over its ceiling fails the guard, and so".to_owned(),
        "# does a ceiling above what the file needs. Regenerate with `cargo xtask ceilings`"
            .to_owned(),
        "# once a file has actually got shorter.".to_owned(),
        String::new(),
    ];

    let mut capped = Vec::new();
    for area in ["crates", "apps/desktop/src", "web/src", "xtask/src"] {
        for entry in walkdir::WalkDir::new(root.join(area))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.components().any(|c| c.as_os_str() == "gen") {
                continue;
            }
            if !path.extension().is_some_and(|e| e == "rs" || e == "tsx") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            let count = text.lines().count();
            if count < WORTH_CAPPING {
                continue;
            }
            let relative = path.strip_prefix(root).unwrap_or(path);
            capped.push(format!("{} {count}", relative.display()));
        }
    }
    capped.sort();

    let total = capped.len();
    lines.extend(capped);
    std::fs::write(root.join("xtask/ceilings.txt"), lines.join("\n") + "\n")?;
    Ok(total)
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
        } else if lines < ceiling {
            findings.push(Finding {
                file: PathBuf::from(relative),
                line: lines,
                what: format!(
                    "{lines} lines under a ceiling of {ceiling} — lower it to {lines} in \
                     xtask/ceilings.txt. The ratchet only tightens"
                ),
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

    /// The half that makes it a ratchet. Without it the number drifts upward
    /// one bump at a time and nothing ever fails.
    #[test]
    fn the_guard_refuses_a_ceiling_that_was_raised() {
        let dir = sized(80);
        let findings = ceilings_in(ENTRY, dir.path());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].what.contains("only tightens"));
    }

    #[test]
    fn the_guard_is_quiet_when_the_ceiling_is_exact() {
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
