//! Where a session's transcript lives.
//!
//! The path is written down when a session starts, so whatever opens it later
//! does not have to guess how the CLI names its files.
//!
//! Reading what is inside one — the `cost-state` line carries the running
//! total — was written here and called by nothing. A card is quiet about what
//! a hand-driven session spent, and being quiet is better than code that looks
//! like it answers the question.

use std::path::{Path, PathBuf};

/// Where a session's transcript lives.
///
/// The directory is the working directory with every character that is not a
/// letter or digit replaced by `-`. A path with dots, slashes and underscores
/// all collapse the same way.
pub fn transcript_path(home: &Path, cwd: &Path, session_id: &str) -> PathBuf {
    let slug: String = cwd
        .to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    home.join(".claude")
        .join("projects")
        .join(slug)
        .join(format!("{session_id}.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_collapses_everything_that_is_not_alphanumeric() {
        let path = transcript_path(
            Path::new("/home/x"),
            Path::new("/home/x/Work/my.app-2"),
            "abc",
        );
        assert_eq!(
            path,
            Path::new("/home/x/.claude/projects/-home-x-Work-my-app-2/abc.jsonl")
        );
    }
}
