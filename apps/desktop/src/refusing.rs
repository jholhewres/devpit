//! What a file read refuses, and the sentence it hands back.
//!
//! Apart from `files.rs` because these are the decisions and that file is the
//! reading. Each one is a number somebody chose and a sentence somebody reads
//! on screen: worth finding in one place, and worth a test that calls the rule
//! rather than restating the comparison.

/// Read no more than this in one go.
///
/// A file past it is refused with its size rather than truncated: half a file
/// in an editor is a file about to be saved with the other half gone.
pub(crate) const MOST_BYTES: u64 = 2 * 1024 * 1024;

/// The most a picture or a PDF may be to travel inline.
///
/// Smaller than the text ceiling on purpose: a data URL is a third bigger
/// than the bytes it carries, and it crosses the IPC boundary as a string.
pub(crate) const MOST_MEDIA_BYTES: u64 = 8 * 1024 * 1024;

fn megabytes(bytes: u64) -> f64 {
    bytes as f64 / 1_048_576.0
}

/// Why a file is too big to open, or nothing.
pub(crate) fn past_the_ceiling(path: &str, bytes: u64) -> Option<String> {
    if bytes <= MOST_BYTES {
        return None;
    }
    Some(format!(
        "{path} is {:.1} MB — past the {} MB this opens",
        megabytes(bytes),
        MOST_BYTES / 1_048_576
    ))
}

/// Why a picture or a PDF is too big to send inline, or nothing.
pub(crate) fn too_big_to_draw(path: &str, bytes: u64) -> Option<String> {
    if bytes <= MOST_MEDIA_BYTES {
        return None;
    }
    Some(format!(
        "{path} is {:.1} MB — past the {} MB this draws",
        megabytes(bytes),
        MOST_MEDIA_BYTES / 1_048_576
    ))
}

/// Nothing but a regular file gets read.
///
/// The workspace holds the tap FIFOs and the tmux socket, and opening a FIFO
/// for reading blocks until some other process writes to it — a click on one
/// row would stop the reader answering rather than fail it.
pub(crate) fn not_a_file(path: &str) -> String {
    format!("{path} is not a file — nothing to read")
}

#[cfg(test)]
#[path = "refusing_tests.rs"]
mod tests;
