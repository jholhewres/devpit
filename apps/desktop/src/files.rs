//! Reading and writing a file inside a project.
//!
//! Every path goes through `tree::resolve`, which follows symlinks first and
//! then checks containment against the project root. This process runs
//! terminals and writes files: reaching it is reaching the machine, and a
//! relative path from a screen is not a path to trust.

use std::path::Path;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;

use devpit_rpc::{ErrorCode, FileContents, FileKind, RpcError};

use crate::kinds::{kind_of, media_type};

use crate::refusing::{
    not_a_file, past_the_ceiling, too_big_to_draw, MOST_BYTES, MOST_MEDIA_BYTES,
};
use crate::roots::root_of;

pub(crate) fn modified(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_millis() as f64)
        .unwrap_or_default()
}

/// Reads at most the largest ceiling of `resolved`, and the size to judge it
/// by. A file past it is read for its first bytes only, which is all the
/// sniffing needs; the whole of a multi-gigabyte file never reaches memory.
pub(crate) fn read_bounded(resolved: &Path, bytes: u64) -> std::io::Result<(Vec<u8>, u64)> {
    use std::io::Read as _;

    let ceiling = MOST_BYTES.max(MOST_MEDIA_BYTES);
    let file = std::fs::File::open(resolved)?;
    if bytes > ceiling {
        let mut head = Vec::new();
        file.take(512).read_to_end(&mut head)?;
        return Ok((head, bytes));
    }
    match devpit_core::data_files::read_within(file, ceiling)? {
        Some(raw) => Ok((raw, bytes)),
        None => Ok((Vec::new(), ceiling + 1)),
    }
}

/// `file.read` — the text of a file, or why it is not text.
#[tauri::command]
#[specta::specta]
pub async fn file_read(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<FileContents, RpcError> {
    crate::off_main::blocking(move || file_read_now(project_id, worktree_id, path)).await
}

/// [`file_read`], on the calling thread.
pub(crate) fn file_read_now(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<FileContents, RpcError> {
    contents(&root_of(&project_id, worktree_id.as_deref())?, path)
}

/// The same reading, against whichever root the caller is standing in.
///
/// Split out so the workspace browser reads a transcript through exactly this
/// code — the ceilings, the sniffing and the containment check are the part
/// worth having, and a second reader would be a second place for them to
/// drift.
pub(crate) fn contents(root: &Path, path: String) -> Result<FileContents, RpcError> {
    let resolved = devpit_core::tree::resolve(root, &path)
        .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?;

    let meta = std::fs::metadata(&resolved).ok();
    let bytes = meta.as_ref().map(|meta| meta.len()).unwrap_or_default();

    if meta.is_some_and(|meta| !meta.is_file()) {
        return Ok(FileContents {
            full_path: resolved.display().to_string(),
            read_at: modified(&resolved),
            not_shown: Some(not_a_file(&path)),
            kind: FileKind::Binary,
            bytes: bytes as f64,
            text: None,
            data_url: None,
            path,
        });
    }

    // Never more than the most this could show: a file past every ceiling is
    // only sniffed, and one that grew past it since `metadata` is refused as
    // if it had been that size all along.
    let (raw, bytes) =
        read_bounded(&resolved, bytes).map_err(|err| RpcError::internal(err.to_string()))?;
    let head = &raw[..raw.len().min(512)];
    let kind = kind_of(&path, head);

    let mut text = None;
    let mut data_url = None;

    let not_shown = match kind {
        // Its own ceiling, not the text one: a screenshot is past 2 MB more
        // often than not, and was refused as text before this was asked.
        FileKind::Image | FileKind::Pdf => too_big_to_draw(&path, bytes).or_else(|| {
            let media = media_type(&path, kind, head);
            data_url = Some(format!("data:{media};base64,{}", BASE64.encode(&raw)));
            None
        }),
        FileKind::Binary => {
            past_the_ceiling(&path, bytes).or_else(|| Some(format!("{path} is not text")))
        }
        // Refused rather than rendered: a megabyte of bytes drawn as
        // replacement characters is worse than a sentence saying it is not
        // text, and saving it back would corrupt the file.
        _ => past_the_ceiling(&path, bytes).or_else(|| {
            text = String::from_utf8(raw).ok();
            text.is_none().then(|| format!("{path} is not text"))
        }),
    };

    Ok(FileContents {
        full_path: resolved.display().to_string(),
        read_at: modified(&resolved),
        path,
        text,
        not_shown,
        bytes: bytes as f64,
        kind,
        data_url,
    })
}

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
