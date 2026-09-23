//! Listing what the CLI holds for a project and devpit does not.
//!
//! Apart from `threads.rs` because that lists devpit's own conversations,
//! titled with the person's first words on purpose. These have no words of the
//! person devpit ever saw, so the CLI's own title is the only name they have.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use devpit_rpc::{OutsideSession, RpcError};

/// The CLI session ids devpit's own conversations of this project resume.
fn known(sessions: &Path) -> HashSet<String> {
    std::fs::read_dir(sessions)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .filter_map(|path| devpit_agentcli::head::read_head(&path)?.session_id)
        .collect()
}

/// `chat.outside` — sessions of this project started outside devpit.
#[tauri::command]
#[specta::specta]
pub async fn chat_outside(project_id: String) -> Result<Vec<OutsideSession>, RpcError> {
    crate::off_main::blocking(move || chat_outside_now(project_id)).await
}

/// [`chat_outside`], on the calling thread.
pub(crate) fn chat_outside_now(project_id: String) -> Result<Vec<OutsideSession>, RpcError> {
    let store = crate::projects::store()?;
    let (_, root) = crate::projects::locate(&store, &project_id)?;
    let sessions =
        crate::projects::home_of(&store, &devpit_core::Store::root()?, &project_id)?.sessions();
    let installations: Vec<PathBuf> = crate::installations::found()?
        .into_iter()
        .map(|one| one.directory)
        .collect();
    Ok(
        devpit_agentcli::outside::sessions(&installations, &root, &known(&sessions))
            .into_iter()
            .map(|one| OutsideSession {
                session_id: one.session_id,
                title: one.title,
                installation: one.installation.display().to_string(),
                last_at: one.last_at,
            })
            .collect(),
    )
}
