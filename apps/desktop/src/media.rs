//! Video, audio and pictures too big to travel inline, streamed to the viewer.
//!
//! A data URL crosses the IPC as one string, so a 20 MB screenshot stalled it
//! and a video could never seek. The window asks for a token for a file it
//! may read (`media.open`), and the `devpitmedia:` scheme serves that file in
//! the ranges a `<video>` asks for. The token names one resolved path and
//! nothing else: the scheme never reads a path from its URL.

use std::collections::VecDeque;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use devpit_rpc::{ErrorCode, RpcError};
use tauri::http::{Response, StatusCode};

/// The scheme, as the window names it in `convertFileSrc`.
pub(crate) const SCHEME: &str = "devpitmedia";
/// The most one response carries: a player asks again for the rest.
const CHUNK: u64 = 4 * 1024 * 1024;
/// The most a response without a range may be — a picture, read whole.
const WHOLE: u64 = 64 * 1024 * 1024;
/// Tokens kept; the oldest goes first.
const KEPT: usize = 256;

static TOKENS: Mutex<VecDeque<(String, PathBuf)>> = Mutex::new(VecDeque::new());

fn fresh_token() -> Result<String, RpcError> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// A token for `path`, already resolved and allowed.
fn token_for(path: PathBuf) -> Result<String, RpcError> {
    let mut tokens = TOKENS
        .lock()
        .map_err(|_| RpcError::internal("media tokens"))?;
    if let Some((token, _)) = tokens.iter().find(|(_, held)| *held == path) {
        return Ok(token.clone());
    }
    let token = fresh_token()?;
    if tokens.len() >= KEPT {
        tokens.pop_front();
    }
    tokens.push_back((token.clone(), path));
    Ok(token)
}

fn path_of(token: &str) -> Option<PathBuf> {
    TOKENS
        .lock()
        .ok()?
        .iter()
        .find(|(held, _)| held == token)
        .map(|(_, path)| path.clone())
}

/// `media.open` — a token the window streams a file by: a project's file by
/// its relative path, or an absolute one the viewer may read.
#[tauri::command]
#[specta::specta]
pub async fn media_open(project_id: Option<String>, path: String) -> Result<String, RpcError> {
    crate::off_main::blocking(move || {
        let resolved = if Path::new(&path).is_absolute() {
            crate::reading_path::viewable(&path)?
        } else {
            let project_id = project_id.ok_or_else(|| {
                RpcError::new(ErrorCode::Invalid, "a relative path needs its project")
            })?;
            let root = crate::roots::root_of(&project_id, None)?;
            devpit_core::tree::resolve(&root, &path)
                .map_err(|err| RpcError::new(ErrorCode::Forbidden, err.to_string()))?
        };
        if !resolved.is_file() {
            return Err(RpcError::new(ErrorCode::Invalid, "that is not a file"));
        }
        token_for(resolved)
    })
    .await
}

/// The bytes a `Range` header asks for, as an inclusive span within `size`,
/// cut to one chunk. `None` for a range that cannot be served.
pub(crate) fn span(header: &str, size: u64) -> Option<(u64, u64)> {
    let (start, end) = header.trim().strip_prefix("bytes=")?.split_once('-')?;
    if start.contains(',') || end.contains(',') || size == 0 {
        return None;
    }
    let (start, end) = match (start.trim(), end.trim()) {
        // The last `n` bytes.
        ("", last) => {
            let last: u64 = last.parse().ok()?;
            (size.saturating_sub(last), size - 1)
        }
        (start, "") => (start.parse().ok()?, size - 1),
        (start, end) => (start.parse().ok()?, end.parse::<u64>().ok()?.min(size - 1)),
    };
    (start <= end && start < size).then(|| (start, end.min(start + CHUNK - 1)))
}

fn answer(status: StatusCode, body: Vec<u8>) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header("X-Content-Type-Options", "nosniff")
        .body(body)
        .unwrap_or_default()
}

/// What the scheme answers for `/<token>`, with the request's `Range` if any.
pub(crate) fn serve(uri_path: &str, range: Option<&str>) -> Response<Vec<u8>> {
    let token = uri_path.trim_start_matches('/');
    let Some(path) = path_of(token) else {
        return answer(StatusCode::NOT_FOUND, b"gone".to_vec());
    };
    let Ok(mut file) = std::fs::File::open(&path) else {
        return answer(StatusCode::NOT_FOUND, b"gone".to_vec());
    };
    let size = file.metadata().map(|meta| meta.len()).unwrap_or(0);
    let mut head = [0u8; 512];
    let read = file.read(&mut head).unwrap_or(0);
    let kind = crate::kinds::stream_type(&path.to_string_lossy(), &head[..read]);
    let (start, end, status) = match range {
        Some(header) => match span(header, size) {
            Some((start, end)) => (start, end, StatusCode::PARTIAL_CONTENT),
            None => {
                return Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header("Content-Range", format!("bytes */{size}"))
                    .body(Vec::new())
                    .unwrap_or_default()
            }
        },
        None if size <= WHOLE => (0, size.saturating_sub(1), StatusCode::OK),
        None => return answer(StatusCode::PAYLOAD_TOO_LARGE, b"ask for a range".to_vec()),
    };
    let length = if size == 0 { 0 } else { end - start + 1 };
    let mut body = vec![0u8; length as usize];
    if file.seek(SeekFrom::Start(start)).is_err() || file.read_exact(&mut body).is_err() {
        return answer(StatusCode::INTERNAL_SERVER_ERROR, Vec::new());
    }
    let mut response = Response::builder()
        .status(status)
        .header("Content-Type", kind)
        .header("Accept-Ranges", "bytes")
        .header("Content-Length", length.to_string())
        .header("X-Content-Type-Options", "nosniff");
    if status == StatusCode::PARTIAL_CONTENT {
        response = response.header("Content-Range", format!("bytes {start}-{end}/{size}"));
    }
    response.body(body).unwrap_or_default()
}

#[cfg(test)]
#[path = "media_tests.rs"]
mod tests;
