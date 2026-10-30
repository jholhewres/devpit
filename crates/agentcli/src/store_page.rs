//! A page of a conversation, read from the end of its file backwards: the
//! last messages of a long transcript cost those messages, not the file.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use devpit_rpc::Message;

/// How much is read at a time, from the end towards the start.
pub const BLOCK: usize = 64 * 1024;
/// A line longer than this is passed over, not held: one runaway output must
/// not take the memory a chat opens with.
pub const LONGEST: usize = 64 * 1024 * 1024;

/// Only what finding the anchor needs: a message's id, not its parts.
#[derive(serde::Deserialize)]
struct Id {
    id: String,
}

/// A page: up to `most` messages, oldest first, whether older ones are left,
/// and how many lines it passed over that did not load.
#[derive(Debug, Default)]
pub struct Page {
    pub messages: Vec<Message>,
    pub older: bool,
    pub skipped: usize,
}

/// Up to `most` messages before the one named `before`, or before the end
/// when `None`. A `before` that is not in the file gives an empty page.
pub fn read_page(path: &Path, before: Option<&str>, most: usize) -> Page {
    let mut page = Page::default();
    let Ok(mut lines) = Backwards::open(path) else {
        return page;
    };
    let mut found = before.is_none();
    while let Some(line) = lines.next_line() {
        let Some(line) = line else {
            // Longer than LONGEST: passed over, said so.
            page.skipped += usize::from(found);
            continue;
        };
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        if !found {
            found = serde_json::from_slice::<Id>(&line)
                .is_ok_and(|one| before == Some(one.id.as_str()));
            continue;
        }
        let Ok(message) = serde_json::from_slice::<Message>(&line) else {
            page.skipped += 1;
            continue;
        };
        if page.messages.len() == most {
            page.older = true;
            break;
        }
        page.messages.push(message);
    }
    page.messages.reverse();
    page
}

/// A file's lines, last first, each read once however many blocks it spans.
struct Backwards {
    file: File,
    /// Where the bytes not yet read end.
    at: u64,
    /// Bytes read that begin no line yet, in file order.
    pending: VecDeque<Vec<u8>>,
    /// How many bytes `pending` holds, and whether the line went past LONGEST.
    held: usize,
    too_long: bool,
    /// Lines found in the current block, ready to hand out, last first;
    /// `None` for one that was too long to keep.
    ready: Vec<Option<Vec<u8>>>,
}

impl Backwards {
    fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let at = file.metadata()?.len();
        Ok(Self {
            file,
            at,
            pending: VecDeque::new(),
            held: 0,
            too_long: false,
            ready: Vec::new(),
        })
    }

    /// The next line back, `Some(None)` for one too long to keep.
    fn next_line(&mut self) -> Option<Option<Vec<u8>>> {
        loop {
            if let Some(line) = self.ready.pop() {
                return Some(line);
            }
            if self.at == 0 {
                // What is left is the file's first line.
                if self.pending.is_empty() && !self.too_long {
                    return None;
                }
                return Some(self.carried(Vec::new()));
            }
            let start = self.at.saturating_sub(BLOCK as u64);
            let mut block = vec![0; (self.at - start) as usize];
            self.file.seek(SeekFrom::Start(start)).ok()?;
            self.file.read_exact(&mut block).ok()?;
            self.at = start;
            let mut newlines = block
                .iter()
                .enumerate()
                .filter(|(_, byte)| **byte == b'\n')
                .map(|(i, _)| i)
                .rev();
            let Some(last) = newlines.next() else {
                // No line ends here: the whole block is the middle of one.
                self.hold_front(block);
                continue;
            };
            // The bytes after the last newline end the line carried so far.
            let mut lines = vec![self.carried(block[last + 1..].to_vec())];
            let mut end = last;
            for newline in newlines {
                lines.push(Some(block[newline + 1..end].to_vec()));
                end = newline;
            }
            // Before the first newline, a line that may begin in an earlier block.
            self.hold_front(block[..end].to_vec());
            // Handed out last first: `pop` takes from the end.
            lines.reverse();
            self.ready = lines;
        }
    }

    /// Keeps bytes that begin no line yet, unless the line is already too long.
    fn hold_front(&mut self, bytes: Vec<u8>) {
        if self.too_long {
            return;
        }
        self.held += bytes.len();
        if self.held > LONGEST {
            self.too_long = true;
            self.pending.clear();
            self.held = 0;
            return;
        }
        self.pending.push_front(bytes);
    }

    /// The line carried so far, ended by `head`, and the carry emptied.
    fn carried(&mut self, head: Vec<u8>) -> Option<Vec<u8>> {
        let pending = std::mem::take(&mut self.pending);
        let too_long = std::mem::replace(&mut self.too_long, false);
        self.held = 0;
        if too_long || head.len() + pending.iter().map(Vec::len).sum::<usize>() > LONGEST {
            return None;
        }
        Some(std::iter::once(head).chain(pending).flatten().collect())
    }
}

#[cfg(test)]
#[path = "store_page_tests.rs"]
mod tests;
