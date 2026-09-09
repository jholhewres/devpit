//! `git log`, in a format nothing in a commit message can forge.

use std::path::Path;

use devpit_rpc::Commit;

use crate::{run, GitError};

/// Unit separator between fields, record separator between commits.
///
/// Not a comma, a tab or a pipe: a commit subject may contain any of those,
/// and a subject that splits a record is a subject someone can write on
/// purpose. `\x1f` and `\x1e` cannot appear in a commit message.
const FORMAT: &str = "--format=%h%x1f%s%x1f%an%x1f%at%x1e";

pub fn history(root: &Path, limit: u32) -> Result<Vec<Commit>, GitError> {
    let count = format!("-n{limit}");
    // A repository with no commits exits non-zero here. That is not a failure
    // worth surfacing — it is a new project, and the honest answer is nothing.
    let Ok(raw) = run(root, &["log", &count, FORMAT]) else {
        return Ok(Vec::new());
    };
    Ok(parse(&raw))
}

fn parse(raw: &str) -> Vec<Commit> {
    raw.split('\x1e')
        .map(str::trim_start)
        .filter(|record| !record.is_empty())
        .filter_map(|record| {
            let mut fields = record.split('\x1f');
            Some(Commit {
                sha: fields.next()?.to_owned(),
                subject: fields.next()?.to_owned(),
                author: fields.next()?.to_owned(),
                committed_at: fields.next()?.parse::<i64>().ok()? as f64,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    #[test]
    fn a_subject_full_of_separators_stays_one_commit() {
        // The point of the control characters. A subject like this is exactly
        // what someone writes when quoting a shell command in a commit.
        let raw = "abc1234\x1ffix: a|b,c\td\x1fJhol\x1f1788212214\x1e";
        let parsed = parse(raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].subject, "fix: a|b,c\td");
        assert_eq!(parsed[0].committed_at, 1_788_212_214.0);
    }

    #[test]
    fn reads_a_real_repository_newest_first() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        std::fs::write(dir.path().join("a.txt"), "one\n").expect("write");
        fixture::commit(dir.path(), "first");
        std::fs::write(dir.path().join("b.txt"), "two\n").expect("write");
        fixture::commit(dir.path(), "second");

        let commits = history(dir.path(), 10).expect("history");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "second");
        assert_eq!(commits[0].author, "Test");
        assert!(commits[0].committed_at > 0.0);
    }

    #[test]
    fn a_repository_with_no_commits_answers_nothing_rather_than_failing() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        assert!(history(dir.path(), 10).expect("history").is_empty());
    }
}
