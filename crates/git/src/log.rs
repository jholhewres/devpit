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

/// `skip` older commits before taking `limit`, for paging back through
/// history without re-reading what the screen already has.
pub fn history(root: &Path, limit: u32, skip: u32) -> Result<Vec<Commit>, GitError> {
    let count = format!("-n{limit}");
    let after = format!("--skip={skip}");
    // A repository with no commits exits non-zero here. That is not a failure
    // worth surfacing — it is a new project, and the honest answer is nothing.
    let Ok(raw) = run(root, &["log", &after, &count, FORMAT]) else {
        return Ok(Vec::new());
    };
    Ok(parse(&raw))
}

/// The commits in `range` (`base..HEAD`), newest first, at most `limit`.
/// Nothing, when git cannot read the range: a base that has gone is no
/// commits to show rather than an error to.
pub fn commits_in(root: &Path, range: &str, limit: u32) -> Vec<Commit> {
    let count = format!("-n{limit}");
    run(root, &["log", &count, FORMAT, range, "--"])
        .map(|raw| parse(&raw))
        .unwrap_or_default()
}

/// Every file a commit since `since` (seconds since the epoch, by commit
/// time) touched, once each. Nothing, where git cannot say.
pub fn touched_since(root: &Path, since: u64) -> Vec<String> {
    let after = format!("--since=@{since}");
    let Ok(raw) = run(root, &["log", &after, "--name-only", "--format=", "-z"]) else {
        return Vec::new();
    };
    let mut touched: Vec<String> = raw
        .split(['\0', '\n'])
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect();
    touched.sort();
    touched.dedup();
    touched
}

/// The branch's upstream, as `origin/main`, when it has one.
pub fn upstream_of(root: &Path) -> Option<String> {
    let named = run(
        root,
        &[
            "rev-parse",
            "--abbrev-ref",
            "--symbolic-full-name",
            "@{upstream}",
        ],
    )
    .ok()?;
    let named = named.trim();
    (!named.is_empty()).then(|| named.to_owned())
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

/// The diff one commit introduced.
///
/// `--first-parent` on a merge, so a merge shows what it brought in rather
/// than replaying every commit of the branch it took.
pub fn show(root: &Path, sha: &str) -> Result<String, GitError> {
    run(
        root,
        &["show", "--format=", "--patch", "--first-parent", sha],
    )
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

        let commits = history(dir.path(), 10, 0).expect("history");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].subject, "second");
        assert_eq!(commits[0].author, "Test");
        assert!(commits[0].committed_at > 0.0);
    }

    #[test]
    fn a_repository_with_no_commits_answers_nothing_rather_than_failing() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        assert!(history(dir.path(), 10, 0).expect("history").is_empty());
    }

    #[test]
    fn skip_pages_back_past_what_was_already_read() {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        for subject in ["first", "second", "third"] {
            std::fs::write(dir.path().join("a.txt"), subject).expect("write");
            fixture::commit(dir.path(), subject);
        }

        let first_page = history(dir.path(), 2, 0).expect("history");
        assert_eq!(
            first_page.iter().map(|c| &c.subject).collect::<Vec<_>>(),
            ["third", "second"]
        );

        let next_page = history(dir.path(), 2, 2).expect("history");
        assert_eq!(
            next_page.iter().map(|c| &c.subject).collect::<Vec<_>>(),
            ["first"]
        );
    }
}

#[cfg(test)]
mod range_tests {
    use super::{commits_in, upstream_of};
    use crate::invoke::fixture::{commit, repo};

    #[test]
    fn the_commits_on_top_of_a_base_are_listed_newest_first() {
        let dir = tempfile::tempdir().expect("tempdir");
        repo(dir.path());
        std::fs::write(dir.path().join("a"), "1").expect("write");
        commit(dir.path(), "first");
        let base = crate::head_of(dir.path()).expect("head");
        for (file, message) in [("b", "second"), ("c", "third")] {
            std::fs::write(dir.path().join(file), "1").expect("write");
            commit(dir.path(), message);
        }
        let subjects: Vec<String> = commits_in(dir.path(), &format!("{base}..HEAD"), 20)
            .into_iter()
            .map(|one| one.subject)
            .collect();
        assert_eq!(subjects, ["third", "second"]);
        // A base that is not there is no commits, not an error.
        assert!(commits_in(dir.path(), "nowhere..HEAD", 20).is_empty());
        // A branch with no upstream has none to name.
        assert_eq!(upstream_of(dir.path()), None);
    }

    #[test]
    fn the_files_touched_since_a_time_are_those_of_the_commits_after_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        repo(dir.path());
        for (file, message) in [("a b.txt", "first"), ("c", "second"), ("a b.txt", "third")] {
            std::fs::write(dir.path().join(file), message).expect("write");
            commit(dir.path(), message);
        }
        assert_eq!(super::touched_since(dir.path(), 0), ["a b.txt", "c"]);
        let tomorrow = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("now")
            .as_secs()
            + 86_400;
        assert!(super::touched_since(dir.path(), tomorrow).is_empty());
    }
}
