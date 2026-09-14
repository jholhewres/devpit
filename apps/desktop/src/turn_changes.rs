//! What a chat turn changed in its checkout, reported at the end of the turn.
//!
//! Apart from `chat.rs` because that file is the conversation, and this is a
//! question about the disk. A snapshot is taken before the agent starts and
//! compared after it stops: the edit calls it reported are not the whole
//! story, because anything its shell ran can change files too.

use std::path::Path;
use std::sync::Mutex;

use devpit_rpc::{ChangedFile, Frame, Part};
use tauri::ipc::Channel;

/// The checkout as it is now, or nothing when it is not a git repository — a
/// turn in a plain folder simply has no rollup.
pub(crate) fn before(checkout: &Path) -> Option<String> {
    devpit_git::snapshot(checkout).ok()
}

/// The part saying what changed since `before`, or nothing when nothing did.
pub(crate) fn since(checkout: &Path, before: Option<&str>) -> Option<Part> {
    let before = before?;
    let after = devpit_git::snapshot(checkout).ok()?;
    let files: Vec<ChangedFile> = devpit_git::changed_between(checkout, before, &after)
        .ok()?
        .into_iter()
        .map(|one| ChangedFile {
            path: one.path,
            added: one.added,
            removed: one.removed,
        })
        .collect();
    (!files.is_empty()).then_some(Part::Changes { files })
}

/// Adds the rollup to the answer being built and to the stream, when there is
/// one.
pub(crate) fn report(
    checkout: &Path,
    before: Option<&str>,
    collected: &Mutex<Vec<Part>>,
    sink: &Channel<Frame>,
    message_id: &str,
) {
    let Some(part) = since(checkout, before) else {
        return;
    };
    collected
        .lock()
        .map(|mut held| held.push(part.clone()))
        .ok();
    let _ = sink.send(Frame::Part {
        message_id: message_id.to_owned(),
        part,
    });
}

#[cfg(test)]
#[path = "turn_changes_tests.rs"]
mod tests;
