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
/// The folder is named by `outside::folder_name`, the one spelling of the
/// CLI's rule — the readers and this writer used to disagree about `_`.
///
/// `config_dir` is the CLI's own directory for the account that ran it —
/// `~/.claude` only when nothing named another, see `cli_config`.
pub fn transcript_path(config_dir: &Path, cwd: &Path, session_id: &str) -> PathBuf {
    config_dir
        .join("projects")
        .join(crate::outside::folder_name(cwd))
        .join(format!("{session_id}.jsonl"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_path_collapses_everything_that_is_not_alphanumeric() {
        let path = transcript_path(
            Path::new("/home/x/.claude"),
            Path::new("/home/x/Work/my.app-2"),
            "abc",
        );
        assert_eq!(
            path,
            Path::new("/home/x/.claude/projects/-home-x-Work-my-app-2/abc.jsonl")
        );
    }

    #[test]
    fn a_profile_with_its_own_directory_keeps_its_transcripts_there() {
        let env = [(
            "CLAUDE_CONFIG_DIR".to_owned(),
            "/home/x/.claude-y".to_owned(),
        )];
        let dir = crate::cli_config::config_dir_of(Path::new("/home/x"), &env, None);
        assert_eq!(
            transcript_path(&dir, Path::new("/w"), "abc"),
            Path::new("/home/x/.claude-y/projects/-w/abc.jsonl")
        );
    }
}
