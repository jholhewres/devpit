//! `sessions.search` — finding a conversation of this project by what was said.
//!
//! The index is brought up to date for the project before each query, reading
//! only transcripts that changed since the last look. The first look at a
//! project reads all of its transcripts — measured at up to 187 MB for a single
//! file on the machine this was written on — so the work runs on a blocking
//! thread and never on the window's.

use std::path::{Path, PathBuf};

use devpit_agentcli::transcript_text::said_in;
use devpit_core::store::search_index;
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// Most hits a query answers with. The one being looked for is near the top,
/// and past this a list is a page to scroll rather than an answer.
const MOST_HITS: i64 = 50;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SessionHit {
    pub session_id: String,
    /// `user` or `assistant`.
    pub role: String,
    /// The words around the match, the matched ones between `[` and `]`.
    pub snippet: String,
    /// The installation whose transcript it is.
    pub installation: String,
    /// The devpit conversation that already resumes this session, if one does —
    /// so a hit opens that tab instead of taking the session in a second time.
    pub conversation_id: Option<String>,
}

/// Which devpit conversation of this project resumes each CLI session.
fn held(sessions: &Path) -> std::collections::HashMap<String, String> {
    std::fs::read_dir(sessions)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| {
            let conversation = path.file_stem()?.to_string_lossy().into_owned();
            let session = devpit_agentcli::head::read_head(&path)?.session_id?;
            Some((session, conversation))
        })
        .collect()
}

fn size_and_mtime(path: &Path) -> Option<(i64, i64)> {
    let meta = std::fs::metadata(path).ok()?;
    let mtime = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    Some((meta.len() as i64, mtime))
}

/// Brings the index up to date for one project's transcripts.
pub(crate) fn refresh(
    store: &devpit_core::Store,
    project_id: &str,
    root: &Path,
    installations: &[PathBuf],
) -> Result<(), RpcError> {
    let conn = store.conn();
    let folder = devpit_agentcli::outside::folder_name(root);
    for installation in installations {
        let dir = installation.join("projects").join(&folder);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for path in entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "jsonl"))
        {
            let Some((size, mtime)) = size_and_mtime(&path) else {
                continue;
            };
            let key = path.display().to_string();
            if !search_index::stale(search_index::recorded(conn, &key)?, size, mtime) {
                continue;
            }
            let session_id = path
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
                .unwrap_or_default();
            search_index::replace_file(
                conn,
                &search_index::TranscriptFile {
                    path: &key,
                    session_id: &session_id,
                    project: project_id,
                    installation: &installation.display().to_string(),
                    size,
                    mtime,
                },
                &said_in(&path)
                    .iter()
                    .map(|said| (said.role, said.text.as_str()))
                    .collect::<Vec<_>>(),
            )?;
        }
    }
    Ok(())
}

/// `sessions.search` — hits in this project's conversations for a query.
#[tauri::command]
#[specta::specta]
pub async fn sessions_search(
    project_id: String,
    query: String,
) -> Result<Vec<SessionHit>, RpcError> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    tauri::async_runtime::spawn_blocking(move || {
        let store = crate::projects::store()?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        let installations: Vec<PathBuf> = crate::installations::found()?
            .into_iter()
            .map(|one| one.directory)
            .collect();
        refresh(&store, &project_id, &root, &installations)?;
        let sessions =
            crate::projects::home_of(&store, &devpit_core::Store::root()?, &project_id)?.sessions();
        let held = held(&sessions);
        let hits = search_index::search(store.conn(), &query, Some(&project_id), MOST_HITS)?;
        Ok(hits
            .into_iter()
            .map(|hit| SessionHit {
                installation: Path::new(&hit.path)
                    .ancestors()
                    .nth(3)
                    .map(|dir| dir.display().to_string())
                    .unwrap_or_default(),
                conversation_id: held.get(&hit.session_id).cloned(),
                session_id: hit.session_id,
                role: hit.role,
                snippet: hit.snippet,
            })
            .collect())
    })
    .await
    .map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?
}

#[cfg(test)]
#[path = "session_search_tests.rs"]
mod tests;
