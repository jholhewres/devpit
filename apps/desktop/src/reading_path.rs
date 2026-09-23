//! `path.read` — a file named by its full path, for the viewer.
//!
//! A terminal prints paths — an agent's `Read(/…/shot.png)`, a test's log —
//! and a click on one opens it here. Only where `path.open` would open it:
//! under a registered project, the devpit workspace, or a CLI installation.
//! The read itself is the one `file.read` does, with its ceilings and its
//! sniffing, against the file's own folder.

use devpit_rpc::{ErrorCode, FileContents, RpcError};

/// `path.read` — what is at an absolute path, as `file.read` answers.
#[tauri::command]
#[specta::specta]
pub async fn path_read(path: String) -> Result<FileContents, RpcError> {
    crate::off_main::blocking(move || path_read_now(path)).await
}

/// [`path_read`], on the calling thread.
pub(crate) fn path_read_now(path: String) -> Result<FileContents, RpcError> {
    let target = crate::reveal::allowed(&path)?;
    let (Some(folder), Some(name)) = (target.parent(), target.file_name()) else {
        return Err(RpcError::new(ErrorCode::Invalid, "that is not a file"));
    };
    let mut read = crate::files::contents(folder, name.to_string_lossy().into_owned())?;
    // Named as it was asked for, which is what the tab and its title show.
    read.path = path;
    Ok(read)
}
