//! What a file is, decided before it is drawn.
//!
//! Apart from reading it because it is a different question: reading is about
//! bytes and permissions, this is about which pane the bytes belong in. A
//! wrong answer here is a picture rendered as text.

use std::path::Path;

use devpit_rpc::FileKind;

/// What a file is.
///
/// The first bytes decide before the extension does. A PNG named `notes.md`
/// is a PNG, and drawing it as markdown would fill the pane with mojibake;
/// the extension only settles what the bytes leave open.
pub fn kind_of(path: &str, head: &[u8]) -> FileKind {
    if head.starts_with(b"\x89PNG\r\n\x1a\n")
        || head.starts_with(b"\xff\xd8\xff")
        || head.starts_with(b"GIF87a")
        || head.starts_with(b"GIF89a")
        || (head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP")
    {
        return FileKind::Image;
    }
    if head.starts_with(b"%PDF-") {
        return FileKind::Pdf;
    }
    // Before the NUL test: media is full of them.
    if let Some(kind) = media_kind(&extension_of(path), head) {
        return kind;
    }
    // Text is decided by the bytes too: a NUL in the first block is the oldest
    // and most reliable sign that this is not something to put in an editor.
    if head.contains(&0) {
        return FileKind::Binary;
    }

    match extension_of(path).as_str() {
        "md" | "markdown" | "mdx" => FileKind::Markdown,
        "svg" => FileKind::Image,
        "pdf" => FileKind::Pdf,
        _ => FileKind::Text,
    }
}

fn extension_of(path: &str) -> String {
    Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

const AUDIO: [&str; 8] = ["mp3", "wav", "ogg", "oga", "opus", "flac", "m4a", "aac"];

/// Video or audio, by the container's own signature, the extension settling
/// what a container holds both of (MP4, Ogg).
fn media_kind(extension: &str, head: &[u8]) -> Option<FileKind> {
    let audio_named = AUDIO.contains(&extension);
    if head.len() >= 12 && &head[4..8] == b"ftyp" {
        return Some(if audio_named {
            FileKind::Audio
        } else {
            FileKind::Video
        });
    }
    if head.starts_with(b"\x1a\x45\xdf\xa3") {
        return Some(if audio_named {
            FileKind::Audio
        } else {
            FileKind::Video
        });
    }
    if head.starts_with(b"OggS") {
        return Some(if extension == "ogv" {
            FileKind::Video
        } else {
            FileKind::Audio
        });
    }
    let wave = head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WAVE";
    // An MPEG audio frame: FF FB, F3 or F2.
    let mpeg = head.len() >= 2 && head[0] == 0xff && [0xfb, 0xf3, 0xf2].contains(&head[1]);
    (wave || mpeg || head.starts_with(b"ID3") || head.starts_with(b"fLaC"))
        .then_some(FileKind::Audio)
}

/// The type video and audio are streamed as.
pub fn stream_type(path: &str, head: &[u8]) -> &'static str {
    let extension = extension_of(path);
    match (kind_of(path, head), extension.as_str()) {
        (FileKind::Video, "mov") => "video/quicktime",
        (FileKind::Video, "webm" | "mkv") => "video/webm",
        (FileKind::Video, "ogv") => "video/ogg",
        (FileKind::Video, _) => "video/mp4",
        (FileKind::Audio, "wav") => "audio/wav",
        (FileKind::Audio, "flac") => "audio/flac",
        (FileKind::Audio, "m4a" | "aac") => "audio/mp4",
        (FileKind::Audio, "ogg" | "oga" | "opus") => "audio/ogg",
        (FileKind::Audio, _) => "audio/mpeg",
        (kind, _) => media_type(path, kind, head),
    }
}

/// The `data:` type a picture is served as.
pub fn media_type(path: &str, kind: FileKind, head: &[u8]) -> &'static str {
    if kind == FileKind::Pdf {
        return "application/pdf";
    }
    if head.starts_with(b"\x89PNG") {
        return "image/png";
    }
    if head.starts_with(b"\xff\xd8\xff") {
        return "image/jpeg";
    }
    if head.starts_with(b"GIF8") {
        return "image/gif";
    }
    if head.len() >= 12 && &head[0..4] == b"RIFF" {
        return "image/webp";
    }
    if Path::new(path)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
    {
        return "image/svg+xml";
    }
    "application/octet-stream"
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod tests;
