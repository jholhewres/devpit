//! How a repository writes its commit subjects, read from its own history,
//! and a first draft of the next one written the same way.
//!
//! A draft, never a commit: the person reads it in the message field and
//! edits it. It carries no credit line for any agent or tool.

use std::collections::HashMap;
use std::path::Path;

use crate::log::history;

/// How many of the newest subjects are read.
const READ: u32 = 40;

/// What the history says about how subjects are written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    /// The `type` most of its `type(scope): subject` lines use, when most
    /// of its subjects are written that way.
    pub prefix: Option<String>,
    /// Whether the words after any prefix start in lower case.
    pub lower: bool,
    /// Whether a subject ends with a full stop.
    pub stop: bool,
}

/// `feat`, from `feat: …`, `feat(ui): …` or `feat!: …`.
fn type_of(subject: &str) -> Option<&str> {
    let (head, rest) = subject.split_once(':')?;
    if !rest.starts_with(' ') {
        return None;
    }
    let kind = head.trim_end_matches('!');
    let kind = kind.split_once('(').map_or(
        kind,
        |(kind, scope)| {
            if scope.ends_with(')') {
                kind
            } else {
                ""
            }
        },
    );
    (!kind.is_empty() && kind.len() <= 12 && kind.chars().all(|c| c.is_ascii_lowercase()))
        .then_some(kind)
}

/// The words after any `type:` prefix.
fn words_of(subject: &str) -> &str {
    match type_of(subject) {
        Some(_) => subject
            .split_once(':')
            .map_or(subject, |(_, rest)| rest.trim_start()),
        None => subject,
    }
}

/// Read from subjects, newest first. No history is plain sentences, and a
/// merge's subject is git's, not the repository's, so it does not vote.
pub fn voice_of(subjects: &[String]) -> Voice {
    let subjects: Vec<&String> = subjects
        .iter()
        .filter(|one| !one.starts_with("Merge "))
        .collect();
    let mut kinds: HashMap<&str, usize> = HashMap::new();
    for subject in &subjects {
        if let Some(kind) = type_of(subject) {
            *kinds.entry(kind).or_default() += 1;
        }
    }
    let typed: usize = kinds.values().sum();
    let prefix = (typed * 2 > subjects.len())
        .then(|| {
            kinds
                .into_iter()
                .max_by(|a, b| a.1.cmp(&b.1).then(b.0.cmp(a.0)))
                .map(|(kind, _)| kind.to_owned())
        })
        .flatten();
    let lower = subjects
        .iter()
        .filter(|one| words_of(one).chars().next().is_some_and(char::is_lowercase))
        .count();
    let stop = subjects.iter().filter(|one| one.ends_with('.')).count();
    Voice {
        prefix,
        lower: lower * 2 > subjects.len(),
        stop: stop * 2 > subjects.len(),
    }
}

/// What changed, named: the file, two files, or how many and where.
fn what(paths: &[String]) -> String {
    let name = |path: &String| path.rsplit('/').next().unwrap_or(path).to_owned();
    match paths {
        [] => "the working tree".to_owned(),
        [one] => name(one),
        [one, two] => format!("{} and {}", name(one), name(two)),
        many => {
            let first = many[0].rsplit_once('/').map(|(dir, _)| dir);
            match first.filter(|dir| many.iter().all(|path| path.starts_with(&format!("{dir}/")))) {
                Some(dir) => format!("{} files in {dir}", many.len()),
                None => format!("{} files", many.len()),
            }
        }
    }
}

/// A subject for `paths`, in the voice given.
pub fn draft(voice: &Voice, paths: &[String]) -> String {
    let words = format!("Update {}", what(paths));
    let words = if voice.lower {
        let mut chars = words.chars();
        chars.next().map_or(String::new(), |first| {
            first.to_lowercase().chain(chars).collect()
        })
    } else {
        words
    };
    let stop = if voice.stop { "." } else { "" };
    match &voice.prefix {
        Some(kind) => format!("{kind}: {words}{stop}"),
        None => format!("{words}{stop}"),
    }
}

/// A subject for `paths` in the voice of the repository at `root`.
pub fn draft_for(root: &Path, paths: &[String]) -> String {
    let subjects: Vec<String> = history(root, READ, 0)
        .unwrap_or_default()
        .into_iter()
        .map(|commit| commit.subject)
        .collect();
    draft(&voice_of(&subjects), paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture;

    fn history_of(subjects: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        fixture::repo(dir.path());
        for (at, subject) in subjects.iter().enumerate() {
            std::fs::write(dir.path().join("f.txt"), at.to_string()).expect("write");
            fixture::commit(dir.path(), subject);
        }
        dir
    }

    fn paths(all: &[&str]) -> Vec<String> {
        all.iter().map(|one| (*one).to_owned()).collect()
    }

    #[test]
    fn a_repo_that_writes_feat_and_fix_gets_a_prefixed_draft() {
        let dir = history_of(&[
            "feat: add the board",
            "fix(ui): keep the caret in place",
            "feat: read the config",
            "chore: bump deps",
            "feat!: drop the old store",
        ]);
        assert_eq!(
            draft_for(dir.path(), &paths(&["src/a.rs"])),
            "feat: update a.rs"
        );
    }

    #[test]
    fn a_repo_that_writes_plain_sentences_gets_a_plain_one() {
        let dir = history_of(&[
            "Add the board",
            "Keep the caret in place.",
            "Read the config.",
            "Bump deps.",
        ]);
        assert_eq!(
            draft_for(dir.path(), &paths(&["src/a.rs", "src/b.rs"])),
            "Update a.rs and b.rs."
        );
    }

    #[test]
    fn a_repo_with_no_history_gets_a_plain_sentence() {
        let dir = history_of(&[]);
        assert_eq!(
            draft_for(dir.path(), &paths(&["web/x.ts", "web/y.ts", "web/z.ts"])),
            "Update 3 files in web"
        );
    }

    #[test]
    fn merges_do_not_outvote_the_repos_own_subjects() {
        let subjects: Vec<String> = [
            "Merge pull request #2 from me/b",
            "feat: add y",
            "Merge pull request #1 from me/a",
            "feat: add x",
        ]
        .map(str::to_owned)
        .into();
        assert_eq!(
            draft(&voice_of(&subjects), &paths(&["src/a.rs"])),
            "feat: update a.rs"
        );
    }

    #[test]
    fn a_colon_in_a_sentence_is_not_a_prefix() {
        assert_eq!(type_of("Note: the board moved"), None);
        assert_eq!(type_of("feat(ui): x"), Some("feat"));
        assert_eq!(type_of("fix!: x"), Some("fix"));
        assert_eq!(type_of("feat:x"), None);
    }
}
