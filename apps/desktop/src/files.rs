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

use crate::roots::root_of;

/// Read no more than this in one go.
///
/// A file past it is refused with its size rather than truncated: half a file
/// in an editor is a file about to be saved with the other half gone.
const MOST_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) fn modified(path: &Path) -> f64 {
    std::fs::metadata(path)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|since| since.as_millis() as f64)
        .unwrap_or_default()
}

/// Why a file is too big to open, or nothing.
///
/// A function of its own for the same reason as `is_stale`: a test that
/// re-states the comparison passes whether or not `file_read` still applies
/// it, and this ceiling is what keeps a window from reading a gigabyte into
/// memory to draw it.
fn past_the_ceiling(path: &str, bytes: u64) -> Option<String> {
    if bytes <= MOST_BYTES {
        return None;
    }
    Some(format!(
        "{path} is {:.1} MB — past the {} MB this opens",
        bytes as f64 / 1_048_576.0,
        MOST_BYTES / 1_048_576
    ))
}

/// The most a picture or a PDF may be to travel inline.
///
/// Smaller than the text ceiling on purpose: a data URL is a third bigger
/// than the bytes it carries, and it crosses the IPC boundary as a string.
const MOST_MEDIA_BYTES: u64 = 8 * 1024 * 1024;

/// `file.read` — the text of a file, or why it is not text.
#[tauri::command]
#[specta::specta]
pub fn file_read(
    project_id: String,
    worktree_id: Option<String>,
    path: String,
) -> Result<FileContents, RpcError> {
    let root = root_of(&project_id, worktree_id.as_deref())?;
    let resolved = devpit_core::tree::resolve(&root, &path)
        .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?;

    let bytes = std::fs::metadata(&resolved)
        .map(|meta| meta.len())
        .unwrap_or_default();

    let raw = match std::fs::read(&resolved) {
        Ok(raw) => raw,
        Err(err) => return Err(RpcError::internal(err.to_string())),
    };
    let head = &raw[..raw.len().min(512)];
    let kind = kind_of(&path, head);

    let mut not_shown = past_the_ceiling(&path, bytes);
    let mut text = None;
    let mut data_url = None;

    match kind {
        FileKind::Image | FileKind::Pdf if not_shown.is_none() => {
            if bytes > MOST_MEDIA_BYTES {
                not_shown = Some(format!(
                    "{path} is {:.1} MB — past the {} MB this draws",
                    bytes as f64 / 1_048_576.0,
                    MOST_MEDIA_BYTES / 1_048_576
                ));
            } else {
                data_url = Some(format!(
                    "data:{};base64,{}",
                    media_type(&path, kind, head),
                    BASE64.encode(&raw)
                ));
            }
        }
        FileKind::Binary => {
            not_shown = not_shown.or_else(|| Some(format!("{path} is not text")));
        }
        _ if not_shown.is_none() => {
            // Refused rather than rendered: a megabyte of bytes drawn as
            // replacement characters is worse than a sentence saying it is not
            // text, and saving it back would corrupt the file.
            text = String::from_utf8(raw).ok();
            if text.is_none() {
                not_shown = Some(format!("{path} is not text"));
            }
        }
        _ => {}
    }

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
