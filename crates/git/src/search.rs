//! `git grep`, for content search across a checkout.
//!
//! No new dependency, and no second parser to keep in sync with git's own:
//! `--untracked` already covers what the tree shows beyond the index, and
//! `invoke::run_diffing` already answers "git is missing" the same way every
//! other command in this crate does. `run_diffing` rather than `run`: `git
//! grep` shares `git diff`'s exit-code contract — 1 means "nothing matched",
//! not "it broke" — and `run` treats any non-zero exit as failure.
//!
//! `-z` rather than the default format: `git grep` puts a colon between the
//! path, the line number and the text, and a path is free to contain one
//! too. `-z` replaces that colon with `\0` — verified against the git on
//! this machine, since its own docs are easy to misread as also changing the
//! line terminator, and it does not: each record still ends in `\n`. A path
//! cannot contain `\0` either way, which is the same reasoning `invoke.rs`
//! gives for `-c core.quotepath=false` three lines away.
//!
//! `-m` bounds how many lines git itself will emit per file. `run_diffing`
//! calls `Command::output`, which buffers everything before this module ever
//! sees a byte — so the ceiling that matters most is the one in the argv,
//! not the one in `parse`. `-m` is a per-file bound, not a global one: `git
//! grep` has no flag for "stop after N matches total", so a pattern common
//! across thousands of files can still produce a large buffer. That residual
//! case is accepted rather than solved here; a global bound would mean
//! reading git's stdout incrementally and killing the process early, which
//! is a bigger change than this story asked for.
//!
//! `git grep --untracked` never reaches a gitignored file, though — the tree
//! draws `dist/`, `.env` and the like regardless. That gap is real and is not
//! papered over here: the caller is told so it can tell the screen.

use std::path::Path;

use crate::{run_diffing, GitError};

/// Refused before git is ever asked, so a pathological pattern cannot turn
/// into an unbounded regex workload. A search term is a few words; a few
/// hundred bytes is generous room past that.
pub const MAX_PATTERN_BYTES: usize = 500;

/// A matched line longer than this becomes a preview. One minified file
/// otherwise turns a single hit into a line the width of a book.
pub const MAX_LINE_CHARS: usize = 300;

/// How many matches git itself will report from one file, via `-m`. This is
/// the ceiling that guards the allocation `run_diffing` makes: it bounds what
/// git writes to its own stdout, before any of it reaches this process.
///
/// `pub(crate)` rather than private: `search_tests.rs` builds a fixture sized
/// exactly to this constant to prove git enforces it, not `parse`.
pub(crate) const MAX_MATCHES_PER_FILE: usize = 500;

#[derive(Debug, Clone, Copy, Default)]
pub struct SearchFlags {
    pub case: bool,
    pub word: bool,
    pub regex: bool,
}

#[derive(Debug, Clone)]
pub struct GrepHit {
    pub path: String,
    pub line: u32,
    pub text: String,
}

#[derive(Debug, Clone, Default)]
pub struct GrepOutcome {
    pub hits: Vec<GrepHit>,
    /// How many lines actually matched, before `ceiling` cut the collection
    /// short. Counted regardless of whether a `GrepHit` was kept for it, so
    /// the caller learns the true size even when `hits` cannot carry it.
    pub matched: usize,
}

/// Maps flags to the `git grep` options that produce them.
///
/// `-i` is the *absence* of match-case: the box on screen reads "match case",
/// and grep's own default is already case-insensitive-off, so the flag this
/// function adds is the negative of what is checked.
///
/// `pub(crate)`, tested directly in `search_tests.rs` for every combination
/// rather than only through `grep`'s end-to-end behaviour.
pub(crate) fn args_for(flags: SearchFlags) -> Vec<&'static str> {
    let mut argv = vec!["grep", "--untracked", "-I", "-n", "-z"];
    if !flags.case {
        argv.push("-i");
    }
    if flags.word {
        argv.push("-w");
    }
    argv.push(if flags.regex { "-E" } else { "-F" });
    argv
}

/// Refuses an oversized pattern before anything is allocated on its behalf —
/// not the argv, not a process, not a line of output.
///
/// `pub(crate)`, called directly from `search_tests.rs` — the rule, not a
/// restatement of the byte count it checks.
pub(crate) fn within_pattern_ceiling(pattern: &str) -> Result<(), GitError> {
    if pattern.len() > MAX_PATTERN_BYTES {
        return Err(GitError::Refused(format!(
            "a search pattern cannot be longer than {MAX_PATTERN_BYTES} bytes"
        )));
    }
    Ok(())
}

/// Searches every tracked and untracked file in `root` for `pattern`.
///
/// `ceiling` governs how many of git's own matches become a `GrepHit`, but it
/// is not the only bound: `-m` in the argv keeps any single file from filling
/// the buffer `run_diffing` reads into memory in the first place, before
/// `ceiling` gets a say. `GrepOutcome::matched` tells the truth about what
/// git actually reported even when `hits` cannot carry all of it.
pub fn grep(
    root: &Path,
    pattern: &str,
    flags: SearchFlags,
    ceiling: usize,
) -> Result<GrepOutcome, GitError> {
    within_pattern_ceiling(pattern)?;
    if pattern.trim().is_empty() {
        return Ok(GrepOutcome::default());
    }

    let per_file = MAX_MATCHES_PER_FILE.to_string();
    let mut argv: Vec<&str> = args_for(flags);
    argv.push("-m");
    argv.push(&per_file);
    argv.push("-e");
    argv.push(pattern);

    let raw = match run_diffing(root, &argv) {
        Ok(raw) => raw,
        // `git grep` exits 1 with nothing on either stream when nothing
        // matched — that is an answer, not a failure. A real failure (a
        // broken `-E` pattern, for instance) always has something to say on
        // stderr, which is what tells the two apart here.
        Err(GitError::Failed { stderr, .. }) if stderr.is_empty() => {
            return Ok(GrepOutcome::default());
        }
        Err(other) => return Err(other),
    };

    Ok(parse(&raw, ceiling))
}

/// Reads `-z` output: one `path\0line\0text` record per `\n`-terminated
/// line, same as the default format except `\0` stands in for the `:` that
/// a path could otherwise contain.
fn parse(raw: &str, ceiling: usize) -> GrepOutcome {
    let mut hits = Vec::new();
    let mut matched = 0usize;

    for line in raw.lines() {
        let mut fields = line.splitn(3, '\0');
        let (Some(path), Some(number), Some(text)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let Ok(line_number) = number.parse::<u32>() else {
            continue;
        };

        matched += 1;
        if hits.len() < ceiling {
            hits.push(GrepHit {
                path: path.to_owned(),
                line: line_number,
                text: shorten(text),
            });
        }
    }

    GrepOutcome { hits, matched }
}

/// Shortens `text` to `MAX_LINE_CHARS`, marking that it was cut.
fn shorten(text: &str) -> String {
    if text.chars().count() <= MAX_LINE_CHARS {
        return text.to_owned();
    }
    let mut short: String = text.chars().take(MAX_LINE_CHARS).collect();
    short.push('…');
    short
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
