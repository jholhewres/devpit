//! A page of a conversation, read from the end of its file backwards: the
//! last messages of a long transcript cost those messages, not the file.

use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use devpit_rpc::Message;

/// How much is read at a time, from the end towards the start.
pub const BLOCK: usize = 64 * 1024;

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
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let Ok(message) = serde_json::from_slice::<Message>(&line) else {
            page.skipped += usize::from(found);
            continue;
        };
        if !found {
            found = before == Some(message.id.as_str());
            continue;
        }
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
    /// Lines found in the current block, ready to hand out, last first.
    ready: Vec<Vec<u8>>,
}

impl Backwards {
    fn open(path: &Path) -> std::io::Result<Self> {
        let file = File::open(path)?;
        let at = file.metadata()?.len();
        Ok(Self {
            file,
            at,
            pending: VecDeque::new(),
            ready: Vec::new(),
        })
    }

    fn next_line(&mut self) -> Option<Vec<u8>> {
        loop {
            if let Some(line) = self.ready.pop() {
                return Some(line);
            }
            if self.at == 0 {
                // What is left is the file's first line.
                return (!self.pending.is_empty()).then(|| {
                    std::mem::take(&mut self.pending)
                        .into_iter()
                        .flatten()
                        .collect()
                });
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
                self.pending.push_front(block);
                continue;
            };
            // The bytes after the last newline end the line carried so far.
            let mut carried = std::mem::take(&mut self.pending);
            carried.push_front(block[last + 1..].to_vec());
            let mut lines = vec![carried.into_iter().flatten().collect::<Vec<u8>>()];
            let mut end = last;
            for newline in newlines {
                lines.push(block[newline + 1..end].to_vec());
                end = newline;
            }
            // Before the first newline, a line that may begin in an earlier block.
            self.pending.push_back(block[..end].to_vec());
            // Handed out last first: `pop` takes from the end.
            lines.reverse();
            self.ready = lines;
        }
    }
}

#[cfg(test)]
#[path = "store_page_tests.rs"]
mod tests;
