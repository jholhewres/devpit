//! A picture pasted into the composer, kept where the agent can read it.
//!
//! Written into the devpit workspace and never into the project: a paste is
//! not a file anybody asked to commit, and a repository that grows a folder of
//! screenshots every time someone shares one is a repository being littered.

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use devpit_rpc::{Attachment, ErrorCode, RpcError};

/// The most a pasted picture may be. Checked on the encoded length before it
/// is decoded, so an oversized paste is refused without ever being held whole.
const MOST_BYTES: usize = 8 * 1024 * 1024;

/// The extension a pasted type is saved under, or nothing for a type this
/// does not take — only pictures, because that is all the composer offers.
pub(crate) fn extension_for(media_type: &str) -> Option<&'static str> {
    match media_type {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        _ => None,
    }
}

/// Decodes a paste, refusing it before allocation when it is too big.
pub(crate) fn decoded(data: &str) -> Result<Vec<u8>, RpcError> {
    // Base64 is four characters for three bytes.
    if data.len() / 4 * 3 > MOST_BYTES {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!(
                "that picture is past the {} MB a paste takes",
                MOST_BYTES / 1_048_576
            ),
        ));
    }
    BASE64
        .decode(data)
        .map_err(|_| RpcError::new(ErrorCode::Invalid, "that paste is not a picture"))
}

/// `chat.paste` — keeps a pasted picture and answers with it as an attachment.
#[tauri::command]
#[specta::specta]
pub fn chat_paste(
    project_id: String,
    media_type: String,
    data: String,
) -> Result<Attachment, RpcError> {
    // The id names a folder; a separator in it would name another one.
    if project_id.contains(['/', '\\']) || project_id.starts_with('.') {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a project"));
    }
    let ext = extension_for(&media_type)
        .ok_or_else(|| RpcError::new(ErrorCode::Invalid, "only pictures can be pasted"))?;
    let bytes = decoded(&data)?;
    let dir = crate::projects::project_home(&project_id)?.pasted();
    std::fs::create_dir_all(&dir).map_err(|err| RpcError::internal(err.to_string()))?;
    let name = format!(
        "pasted-{}.{ext}",
        ulid::Ulid::generate().to_string().to_lowercase()
    );
    let path = dir.join(&name);
    std::fs::write(&path, bytes).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(Attachment {
        name,
        // Absolute: the file is outside the project, and the agent reads `@`
        // paths as they are written.
        path: path.display().to_string(),
        kind: ext.to_owned(),
    })
}

#[cfg(test)]
#[path = "pasting_tests.rs"]
mod tests;
