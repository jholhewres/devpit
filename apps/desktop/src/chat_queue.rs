//! What waits to be said in a conversation once its turn is over.
//!
//! A file of its own rather than a field in the head: every turn rewrites the
//! head, and a queue written between two of those writes would be lost to the
//! second. In a folder of its own, so nothing that lists the heads reads one.

use std::io::Read as _;
use std::path::{Path, PathBuf};

use devpit_rpc::{ChatQueue, ErrorCode, RpcError};

/// The most a queue file may hold. A paste too long to type is an attachment,
/// so this is many messages of typing, read whole only below it.
pub(crate) const MOST_BYTES: u64 = 4 * 1024 * 1024;
/// The most messages one conversation may hold waiting.
pub(crate) const MOST_ITEMS: usize = 100;

/// An id that names a file, so it may not be a path.
fn plain(id: &str) -> bool {
    !id.is_empty() && !id.contains(['/', '\\']) && !id.starts_with('.')
}

pub(crate) fn queue_path(sessions: &Path, conversation_id: &str) -> PathBuf {
    sessions
        .join("queued")
        .join(format!("{conversation_id}.json"))
}

/// The queue kept at `path`; empty when there is none, or it does not read.
pub(crate) fn read_at(path: &Path) -> ChatQueue {
    let Ok(file) = std::fs::File::open(path) else {
        return ChatQueue::default();
    };
    let mut text = String::new();
    // One byte past the ceiling says the file is over it, without holding more.
    if file.take(MOST_BYTES + 1).read_to_string(&mut text).is_err()
        || text.len() as u64 > MOST_BYTES
    {
        return ChatQueue::default();
    }
    serde_json::from_str(&text).unwrap_or_default()
}

/// Writes `queue` at `path` whole, through a file beside it, so a crash mid-write
/// leaves the last queue rather than half of this one. An empty queue is no file.
pub(crate) fn write_at(path: &Path, queue: &ChatQueue) -> Result<(), RpcError> {
    let too_big = |what: String| RpcError::new(ErrorCode::Invalid, what);
    if queue.items.len() > MOST_ITEMS {
        return Err(too_big(format!(
            "a queue holds at most {MOST_ITEMS} messages"
        )));
    }
    if queue.items.is_empty() {
        return match std::fs::remove_file(path) {
            Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                Err(RpcError::internal(err.to_string()))
            }
            _ => Ok(()),
        };
    }
    let text = serde_json::to_string(queue).map_err(|err| RpcError::internal(err.to_string()))?;
    if text.len() as u64 > MOST_BYTES {
        return Err(too_big("that queue is too long to keep".to_owned()));
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| RpcError::internal(err.to_string()))?;
    }
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, text).map_err(|err| RpcError::internal(err.to_string()))?;
    std::fs::rename(&partial, path).map_err(|err| RpcError::internal(err.to_string()))
}

fn path_of(project_id: &str, conversation_id: &str) -> Result<PathBuf, RpcError> {
    if !plain(conversation_id) {
        return Err(RpcError::new(
            ErrorCode::Forbidden,
            "that is not a conversation",
        ));
    }
    let sessions = crate::projects::project_home(project_id)?.sessions();
    Ok(queue_path(&sessions, conversation_id))
}

/// `chat.queue` — what waits in a conversation, as it was last kept.
#[tauri::command]
#[specta::specta]
pub async fn chat_queue(
    project_id: String,
    conversation_id: String,
) -> Result<ChatQueue, RpcError> {
    crate::off_main::blocking(move || Ok(read_at(&path_of(&project_id, &conversation_id)?))).await
}

/// `chat.queue_keep` — keeps what waits in a conversation, whole, in place of
/// what was kept before. Answers with what was kept.
#[tauri::command]
#[specta::specta]
pub async fn chat_queue_keep(
    project_id: String,
    conversation_id: String,
    queue: ChatQueue,
) -> Result<ChatQueue, RpcError> {
    crate::off_main::blocking(move || {
        write_at(&path_of(&project_id, &conversation_id)?, &queue)?;
        Ok(queue)
    })
    .await
}

#[cfg(test)]
#[path = "chat_queue_tests.rs"]
mod tests;
