//! `project.search` — content search over a checkout, through `git grep`.
//!
//! Apart from `projects.rs` on purpose: that file is at its own ceiling, and
//! this command carries truncation and grouping logic besides.

use devpit_git::{GrepHit, SearchFlags};
use devpit_rpc::{ErrorCode, RpcError, SearchFile, SearchHits, SearchLine};

use crate::projects::{checkout, locate, store};

/// Hits are capped in the thousands: past this, the memory to hold every
/// match stops being worth what one more match would tell the reader.
const MAX_HITS: usize = 2_000;

/// `project.search` — every line matching `pattern`, grouped by file.
///
/// `match_case`, `word` and `regex` mirror the toggles the search field
/// already has for name search, so flipping one behaves the same way in both
/// modes. Named `match_case` rather than `case`: the latter is a reserved
/// word, and specta emits it as a bare parameter name that no JavaScript
/// engine can parse.
#[tauri::command]
#[specta::specta]
pub fn project_search(
    project_id: String,
    worktree_id: Option<String>,
    pattern: String,
    match_case: bool,
    word: bool,
    regex: bool,
) -> Result<SearchHits, RpcError> {
    let store = store()?;
    let (_, root) = locate(&store, &project_id)?;
    let root = checkout(&root, worktree_id.as_deref());

    let flags = SearchFlags {
        case: match_case,
        word,
        regex,
    };
    let outcome = devpit_git::grep(&root, &pattern, flags, MAX_HITS).map_err(|err| match err {
        // Refused before git ran at all — the person can shorten the
        // pattern, so this is theirs to act on rather than an internal one.
        devpit_git::GitError::Refused(message) => RpcError::new(ErrorCode::Invalid, message),
        other => RpcError::internal(other.to_string()),
    })?;

    let matched = outcome.matched as u32;
    let files = group(outcome.hits);
    let carried: usize = files.iter().map(|file| file.lines.len()).sum();

    Ok(SearchHits {
        truncated: outcome.matched > carried,
        matched,
        files,
    })
}

/// Groups hits by file in the order `git grep` already produced them —
/// contiguous per file — rather than re-sorting what the backend already
/// knew.
fn group(hits: Vec<GrepHit>) -> Vec<SearchFile> {
    let mut files: Vec<SearchFile> = Vec::new();
    for hit in hits {
        let line = SearchLine {
            line: hit.line,
            text: hit.text,
        };
        match files.last_mut() {
            Some(file) if file.path == hit.path => file.lines.push(line),
            _ => files.push(SearchFile {
                path: hit.path,
                lines: vec![line],
            }),
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(path: &str, line: u32) -> GrepHit {
        GrepHit {
            path: path.to_owned(),
            line,
            text: "text".to_owned(),
        }
    }

    #[test]
    fn consecutive_hits_from_the_same_file_share_one_entry() {
        let files = group(vec![hit("a.rs", 1), hit("a.rs", 4), hit("b.rs", 2)]);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.rs");
        assert_eq!(files[0].lines.len(), 2);
        assert_eq!(files[1].path, "b.rs");
        assert_eq!(files[1].lines.len(), 1);
    }

    #[test]
    fn the_same_file_appearing_twice_non_consecutively_stays_two_entries() {
        // Never happens from real `git grep` output, but the function must
        // not silently merge what is not adjacent.
        let files = group(vec![hit("a.rs", 1), hit("b.rs", 2), hit("a.rs", 9)]);
        assert_eq!(files.len(), 3);
    }
}
