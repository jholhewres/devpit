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
pub(crate) const MOST_BYTES: usize = 8 * 1024 * 1024;

/// The extension a pasted type is saved under, or nothing for a type this
/// does not take — pictures, and the voice messages the composer records.
/// A recording's type comes with its codec (`audio/webm;codecs=opus`).
pub(crate) fn extension_for(media_type: &str) -> Option<&'static str> {
    match media_type.split(';').next().unwrap_or_default().trim() {
        "image/png" => Some("png"),
        "image/jpeg" => Some("jpg"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "audio/webm" => Some("webm"),
        "audio/ogg" => Some("ogg"),
        "audio/mp4" => Some("m4a"),
        "audio/wav" => Some("wav"),
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
pub async fn chat_paste(
    project_id: String,
    media_type: String,
    data: String,
) -> Result<Attachment, RpcError> {
    crate::off_main::blocking(move || chat_paste_now(project_id, media_type, data)).await
}

/// [`chat_paste`], on the calling thread.
pub(crate) fn chat_paste_now(
    project_id: String,
    media_type: String,
    data: String,
) -> Result<Attachment, RpcError> {
    // The id names a folder; a separator in it would name another one.
    if project_id.contains(['/', '\\']) || project_id.starts_with('.') {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a project"));
    }
    let ext = extension_for(&media_type).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Invalid,
            "only pictures and recordings can be pasted",
        )
    })?;
    let bytes = decoded(&data)?;
    let dir = crate::projects::project_home(&project_id)?.pasted();
    kept_in(&dir, &media_type, ext, &bytes)
}

/// Writes a paste into `dir` under a name devpit makes, never one it was
/// given, so nothing written lands outside it.
pub(crate) fn kept_in(
    dir: &std::path::Path,
    media_type: &str,
    ext: &str,
    bytes: &[u8],
) -> Result<Attachment, RpcError> {
    std::fs::create_dir_all(dir).map_err(|err| RpcError::internal(err.to_string()))?;
    // A recording is named as one, which is what transcribing it asks for.
    let what = if media_type.starts_with("audio/") {
        "voice"
    } else {
        "pasted"
    };
    let name = format!(
        "{what}-{}.{ext}",
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

/// `chat.image` — a picture a sent message names, as a `data:` URL for its
/// thumbnail: a paste kept for the project, or an image in the project.
#[tauri::command]
#[specta::specta]
pub async fn chat_image(project_id: String, path: String) -> Result<String, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        let pasted = crate::projects::project_home(&project_id)?.pasted();
        picture_in(&[root.as_path(), pasted.as_path()], &path)
    })
    .await
}

/// The picture at `path`, relative to the first root or absolute inside any,
/// read only when it is an image under the paste ceiling.
pub(crate) fn picture_in(roots: &[&std::path::Path], path: &str) -> Result<String, RpcError> {
    let refused = || RpcError::new(ErrorCode::Forbidden, "that picture is not the project's");
    let wanted = std::path::Path::new(path);
    let resolved = if wanted.is_absolute() {
        let found = wanted.canonicalize().map_err(|_| refused())?;
        roots
            .iter()
            .filter_map(|root| root.canonicalize().ok())
            .any(|root| found.starts_with(root))
            .then_some(found)
            .ok_or_else(refused)?
    } else {
        let root = roots.first().ok_or_else(refused)?;
        devpit_core::tree::resolve(root, path).map_err(|_| refused())?
    };
    let ext = resolved
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let media = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        _ => return Err(RpcError::new(ErrorCode::Invalid, "that is not a picture")),
    };
    let size = std::fs::metadata(&resolved).map_err(|_| refused())?.len();
    if size > MOST_BYTES as u64 {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "that picture is too big to show",
        ));
    }
    let bytes = std::fs::read(&resolved).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(format!("data:{media};base64,{}", BASE64.encode(bytes)))
}

#[cfg(test)]
#[path = "pasting_tests.rs"]
mod tests;
