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
    // Text is decided by the bytes too: a NUL in the first block is the oldest
    // and most reliable sign that this is not something to put in an editor.
    if head.contains(&0) {
        return FileKind::Binary;
    }

    let extension = Path::new(path)
        .extension()
        .map(|ext| ext.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "md" | "markdown" | "mdx" => FileKind::Markdown,
        "svg" => FileKind::Image,
        "pdf" => FileKind::Pdf,
        _ => FileKind::Text,
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
