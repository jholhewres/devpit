//! `chat.attach` — a file dropped on the composer, as a path the agent can
//! be pointed at.

use std::path::PathBuf;

use devpit_rpc::{Attachment, ErrorCode, RpcError};

/// `chat.attach` — a dropped file, as something the agent can be pointed at.
///
/// The absolute path never reaches the screen or the prompt: it says nothing
/// on another machine, and a path outside the project is refused here rather
/// than read.
#[tauri::command]
#[specta::specta]
pub async fn chat_attach(project_id: String, path: String) -> Result<Attachment, RpcError> {
    crate::off_main::blocking(move || chat_attach_now(project_id, path)).await
}

/// [`chat_attach`], on the calling thread.
pub(crate) fn chat_attach_now(project_id: String, path: String) -> Result<Attachment, RpcError> {
    let store = crate::projects::store()?;
    let (_, root) = crate::projects::locate(&store, &project_id)?;
    let absolute = PathBuf::from(&path);
    let relative = devpit_core::tree::relative_to(&root, &absolute)
        .map_err(|_| RpcError::new(ErrorCode::Invalid, "that file is not in the project"))?;
    Ok(Attachment {
        name: absolute
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative.clone()),
        kind: absolute
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
        path: relative,
    })
}
