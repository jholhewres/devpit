//! Who made a checkout, and which kinds a person wants to see.
//!
//! `git worktree list` is honest and unhelpful: it returns every worktree of
//! the repository, and somebody working with agents has three kinds at once —
//! devpit's own card checkouts, the ones Claude Code makes for itself, and
//! whatever they made by hand. Added up into one number, the project row says
//! "7 worktrees" about a repository they made none of.
//!
//! So each one is classified by where it sits, and the lists can be about one
//! kind at a time.

use std::collections::BTreeSet;
use std::path::Path;

use devpit_rpc::WorktreeOrigin;

/// What an agent's own worktree folder is called, under the project.
///
/// A path fragment rather than a prefix: Claude Code puts them at
/// `<project>/.claude/worktrees/<name>`, and a person who moved their project
/// still has that shape inside it.
const AGENT_FOLDERS: &[(&str, &str)] = &[(".claude", "worktrees")];

/// Who made this checkout.
///
/// `mine` is the folders devpit creates card worktrees in — the workspace
/// default and whatever the setting names. Compared as resolved paths: two
/// spellings of one folder are one folder, and the symlink case is exactly the
/// one that would classify a devpit worktree as somebody else's.
pub fn origin_of(path: &Path, mine: &[std::path::PathBuf]) -> WorktreeOrigin {
    let here = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    for base in mine {
        let base = std::fs::canonicalize(base).unwrap_or_else(|_| base.clone());
        if here.starts_with(&base) {
            return WorktreeOrigin::Devpit;
        }
    }

    let parts: Vec<&std::ffi::OsStr> = here.iter().collect();
    for (folder, under) in AGENT_FOLDERS {
        let found = parts
            .windows(2)
            .any(|pair| pair[0] == *folder && pair[1] == *under);
        if found {
            return WorktreeOrigin::Claude;
        }
    }

    WorktreeOrigin::Other
}

/// The short word a preference stores an origin as.
pub fn word_of(origin: WorktreeOrigin) -> &'static str {
    match origin {
        WorktreeOrigin::Devpit => "devpit",
        WorktreeOrigin::Claude => "claude",
        WorktreeOrigin::Other => "other",
    }
}

/// Reads the hidden set out of a preference.
///
/// A word this build does not know is dropped rather than kept: the set is
/// what gets hidden, and a typo that hid nothing is better than one that hid
/// everything.
pub fn hidden_in(stored: &str) -> BTreeSet<&'static str> {
    stored
        .split(',')
        .map(str::trim)
        .filter_map(|word| {
            [
                WorktreeOrigin::Devpit,
                WorktreeOrigin::Claude,
                WorktreeOrigin::Other,
            ]
            .into_iter()
            .map(word_of)
            .find(|known| *known == word)
        })
        .collect()
}

/// Writes one back, in a stable order so two equal sets compare equal.
pub fn hidden_as(hidden: &BTreeSet<&'static str>) -> String {
    hidden.iter().copied().collect::<Vec<_>>().join(",")
}

/// Whether this origin is shown, given what was stored.
pub fn shown(stored: &str, origin: WorktreeOrigin) -> bool {
    !hidden_in(stored).contains(word_of(origin))
}

#[cfg(test)]
#[path = "sourcing_tests.rs"]
mod tests;
