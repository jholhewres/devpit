//! Finding a conversation by what was said in it.
//!
//! A full-text index over the CLI's own transcripts, kept in the store and
//! filled incrementally. What goes in is what a person would search for — the
//! words they typed and the words the agent answered with — and not tool
//! output, which is most of every transcript by volume and none of what anyone
//! remembers a conversation by.
//!
//! A file is re-read when its size **or** its mtime moved. Size alone is the
//! trap Orca fell into: a transcript rewritten to the length it already had
//! looked unchanged and kept its stale rows.
//!
//! What a transcript line *says* is read in `devpit_agentcli::transcript_text`:
//! the line format is the CLI's, and this crate only stores and finds text.

use rusqlite::{params, Connection};

use crate::store::StoreError;

/// Whether a file must be read again, given what the index last recorded.
pub fn stale(recorded: Option<(i64, i64)>, size: i64, mtime: i64) -> bool {
    recorded != Some((size, mtime))
}

/// One hit, with the matched words marked.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub session_id: String,
    pub path: String,
    pub role: String,
    /// The text around the match, with `[` and `]` around the matched words.
    pub snippet: String,
}

/// Which transcript a set of rows came from, and what it looked like when read.
#[derive(Debug, Clone, PartialEq)]
pub struct TranscriptFile<'a> {
    pub path: &'a str,
    pub session_id: &'a str,
    pub project: &'a str,
    pub installation: &'a str,
    pub size: i64,
    pub mtime: i64,
}

/// Replaces what the index holds for one file.
pub fn replace_file(
    conn: &Connection,
    file: &TranscriptFile<'_>,
    // (role, text) pairs, as the transcript reader found them.
    lines: &[(&str, &str)],
) -> Result<(), StoreError> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "DELETE FROM session_text WHERE path = ?1",
        params![file.path],
    )?;
    for (role, text) in lines {
        tx.execute(
            "INSERT INTO session_text (path, session_id, project, installation, role, text) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                file.path,
                file.session_id,
                file.project,
                file.installation,
                role,
                text
            ],
        )?;
    }
    tx.execute(
        "INSERT INTO session_file (path, size, mtime) VALUES (?1, ?2, ?3) \
         ON CONFLICT(path) DO UPDATE SET size = excluded.size, mtime = excluded.mtime",
        params![file.path, file.size, file.mtime],
    )?;
    tx.commit()?;
    Ok(())
}

/// Drops what the index holds under `folder` — one installation's folder of
/// one project's transcripts, read in full just now — for files no longer in
/// it, so a hit never points at a conversation that was deleted.
///
/// Scoped to a folder that was read, never to a project: an installation that
/// could not be read this time keeps its index rather than losing it to a
/// passing error. Found through `session_file`'s key, not the text table.
pub fn forget_missing(
    conn: &Connection,
    folder: &str,
    present: &[String],
) -> Result<usize, StoreError> {
    let prefix = format!("{}/", folder.trim_end_matches('/'));
    let mut statement =
        conn.prepare("SELECT path FROM session_file WHERE path >= ?1 AND path < ?2")?;
    let known: Vec<String> = statement
        .query_map(params![prefix, format!("{prefix}\u{10FFFF}")], |row| {
            row.get(0)
        })?
        .collect::<Result<_, _>>()?;
    let present: std::collections::HashSet<&str> = present.iter().map(String::as_str).collect();
    let gone: Vec<&String> = known
        .iter()
        .filter(|path| !present.contains(path.as_str()))
        .collect();
    if gone.is_empty() {
        return Ok(0);
    }
    let tx = conn.unchecked_transaction()?;
    for path in &gone {
        tx.execute("DELETE FROM session_text WHERE path = ?1", params![path])?;
        tx.execute("DELETE FROM session_file WHERE path = ?1", params![path])?;
    }
    tx.commit()?;
    Ok(gone.len())
}

/// What the index last recorded for a file.
pub fn recorded(conn: &Connection, path: &str) -> Result<Option<(i64, i64)>, StoreError> {
    let mut statement = conn.prepare("SELECT size, mtime FROM session_file WHERE path = ?1")?;
    let mut rows = statement.query(params![path])?;
    Ok(match rows.next()? {
        Some(row) => Some((row.get(0)?, row.get(1)?)),
        None => None,
    })
}

/// A query a person typed, as an FTS5 expression that cannot be a syntax error.
///
/// Each word quoted, so `don't`, `a-b` and `"` are words and not operators; a
/// trailing `*` keeps prefix search working while typing.
pub fn expression(typed: &str) -> Option<String> {
    let words: Vec<String> = typed
        .split_whitespace()
        .map(|word| format!("\"{}\"*", word.replace('"', "\"\"")))
        .collect();
    (!words.is_empty()).then(|| words.join(" "))
}

/// The best matches, filtered to a project when one is given.
pub fn search(
    conn: &Connection,
    typed: &str,
    project: Option<&str>,
    limit: i64,
) -> Result<Vec<Hit>, StoreError> {
    let Some(expression) = expression(typed) else {
        return Ok(Vec::new());
    };
    let mut statement = conn.prepare(
        "SELECT session_id, path, role, snippet(session_text, 5, '[', ']', '…', 12) \
         FROM session_text \
         WHERE session_text MATCH ?1 AND (?2 IS NULL OR project = ?2) \
         ORDER BY rank LIMIT ?3",
    )?;
    let hits = statement
        .query_map(params![expression, project, limit], |row| {
            Ok(Hit {
                session_id: row.get(0)?,
                path: row.get(1)?,
                role: row.get(2)?,
                snippet: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(hits)
}

#[cfg(test)]
#[path = "search_index_tests.rs"]
mod tests;
