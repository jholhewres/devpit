//! What a chat turn changed in its checkout, reported at the end of the turn.
//!
//! Apart from `chat.rs` because that file is the conversation, and this is a
//! question about the disk. A snapshot is taken before the agent starts and
//! compared after it stops: the edit calls it reported are not the whole
//! story, because anything its shell ran can change files too.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    if let Some(part) = since(checkout, before) {
        keep(part, collected, sink, message_id);
    }
}

fn keep(part: Part, collected: &Mutex<Vec<Part>>, sink: &Channel<Frame>, message_id: &str) {
    collected
        .lock()
        .map(|mut held| held.push(part.clone()))
        .ok();
    let _ = sink.send(Frame::Part {
        message_id: message_id.to_owned(),
        part,
    });
}

/// The checkout before a turn, read on a thread of its own: the agent starts
/// while git reads, and only the end waits for it, if it is not done by then.
/// An edit the agent makes before the snapshot is taken counts as the
/// checkout's, not the turn's.
#[derive(Default)]
pub(crate) struct Before(Option<std::thread::JoinHandle<Option<String>>>);

impl Before {
    pub(crate) fn start(checkout: PathBuf) -> Self {
        Self::reading(move || before(&checkout))
    }

    fn reading(snapshot: impl FnOnce() -> Option<String> + Send + 'static) -> Self {
        Self(
            std::thread::Builder::new()
                .name("turn-before".to_owned())
                .spawn(snapshot)
                .ok(),
        )
    }

    /// The snapshot, once git has it.
    pub(crate) fn taken(self) -> Option<String> {
        self.0?.join().ok().flatten()
    }
}

/// [`report`], off the async runtime.
pub(crate) async fn reported(
    checkout: PathBuf,
    before: Before,
    collected: Arc<Mutex<Vec<Part>>>,
    sink: Channel<Frame>,
    message_id: String,
) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        report(
            &checkout,
            before.taken().as_deref(),
            &collected,
            &sink,
            &message_id,
        );
    })
    .await;
}

/// [`ended`], off the async runtime, git reading the checkout after the end is sent.
pub(crate) async fn end_then_changes(
    checkout: PathBuf,
    before: Before,
    end: devpit_rpc::TurnEnd,
    collected: Arc<Mutex<Vec<Part>>>,
    sink: Channel<Frame>,
    message_id: String,
) {
    let _ = tauri::async_runtime::spawn_blocking(move || {
        ended(
            &sink,
            end,
            || since(&checkout, before.taken().as_deref()),
            &collected,
            &message_id,
        );
    })
    .await;
}

/// A turn's last frames: its end first, so the window says the turn is over
/// at once, then what it changed, once `changed` has read it from git.
pub(crate) fn ended(
    sink: &Channel<Frame>,
    end: devpit_rpc::TurnEnd,
    changed: impl FnOnce() -> Option<Part>,
    collected: &Mutex<Vec<Part>>,
    message_id: &str,
) {
    let _ = sink.send(Frame::Ended { end });
    if let Some(part) = changed() {
        keep(part, collected, sink, message_id);
    }
}

#[cfg(test)]
#[path = "turn_changes_tests.rs"]
mod tests;
