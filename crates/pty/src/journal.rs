//! A pane's finished blocks, kept on disk so they outlive the app.
//!
//! tmux keeps a pane running across a restart; the blocks cut from it lived
//! only in memory, so the terminal came back with its commands forgotten. The
//! journal is an append-only file per pane: one record per block as it ends,
//! one per bookmark changed. Appending is what keeps it off the hot path — a
//! block ending costs one write of that block, never a rewrite of the rest —
//! and the file is rewritten only when it has grown to twice what it keeps.
//!
//! A record torn by a crash ends the read there: everything before it is
//! whole, and losing the last block beats refusing the file.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::blocks::{Block, Head, MOST_OUTPUT};

/// How many finished blocks a pane keeps across restarts, and how much output
/// in all. Fewer than it keeps in memory: these are read back on every start.
pub const KEPT_BLOCKS: usize = 200;
pub const KEPT_BYTES: usize = 16 * 1024 * 1024;

/// The first bytes of a journal, and its version.
const MAGIC: &[u8] = b"devpit-blocks 1\n";
const BLOCK: u8 = b'b';
const MARK: u8 = b'm';

/// The longest command or folder read back. The shells cap a line at 2000
/// characters; this is a ceiling on a length the file supplies, not a limit
/// anyone meets.
const MOST_TEXT: usize = 64 * 1024;

/// The most of a file read at all: it is compacted at twice what it keeps, so
/// anything much past that is not a journal this build wrote.
const MOST_FILE: u64 = 4 * KEPT_BYTES as u64;

/// One pane's journal.
#[derive(Debug)]
pub struct Journal {
    path: PathBuf,
    /// Records written since the file was last read or rewritten.
    records: usize,
    bytes: u64,
}

impl Journal {
    /// The journal at `path`. Nothing is read or written until asked.
    pub fn at(path: PathBuf) -> Self {
        let bytes = std::fs::metadata(&path).map(|meta| meta.len()).unwrap_or(0);
        Self {
            path,
            records: 0,
            bytes,
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The blocks it holds, oldest first, bookmarks applied and bounded.
    pub fn load(&mut self) -> Vec<Block> {
        let Ok(file) = std::fs::File::open(&self.path) else {
            return Vec::new();
        };
        let mut bytes = Vec::new();
        if file.take(MOST_FILE).read_to_end(&mut bytes).is_err() {
            return Vec::new();
        }
        let blocks = kept(decode(&bytes));
        self.records = blocks.len();
        self.bytes = bytes.len() as u64;
        blocks
    }

    /// Adds a block that has just ended.
    pub fn append(&mut self, block: &Block) -> std::io::Result<()> {
        let mut record = Vec::with_capacity(block.output.len() + 64);
        encode_block(block, &mut record);
        self.write(&record)
    }

    /// Records a block's bookmark being set or taken away.
    pub fn mark(&mut self, id: u64, on: bool) -> std::io::Result<()> {
        let mut record = vec![MARK];
        record.extend_from_slice(&id.to_le_bytes());
        record.push(u8::from(on));
        self.write(&record)
    }

    /// Forgets every block: the file goes.
    pub fn remove(&mut self) {
        let _ = std::fs::remove_file(&self.path);
        self.records = 0;
        self.bytes = 0;
    }

    fn write(&mut self, record: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file =
            private(std::fs::OpenOptions::new().create(true).append(true)).open(&self.path)?;
        let mut bytes = Vec::with_capacity(record.len() + MAGIC.len());
        if file.metadata()?.len() == 0 {
            bytes.extend_from_slice(MAGIC);
        }
        bytes.extend_from_slice(record);
        // One write: a bookmark and a block ending at once must not interleave.
        file.write_all(&bytes)?;
        self.records += 1;
        self.bytes += bytes.len() as u64;
        if self.records > 2 * KEPT_BLOCKS || self.bytes > 2 * KEPT_BYTES as u64 {
            self.compact()?;
        }
        Ok(())
    }

    /// Rewrites the file with only what it keeps. Written aside and renamed
    /// over, so a crash halfway leaves the old file whole.
    fn compact(&mut self) -> std::io::Result<()> {
        let blocks = self.load();
        let mut bytes = MAGIC.to_vec();
        for block in &blocks {
            encode_block(block, &mut bytes);
        }
        let aside = self.path.with_extension("compacting");
        private(
            std::fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true),
        )
        .open(&aside)?
        .write_all(&bytes)?;
        std::fs::rename(&aside, &self.path)?;
        self.records = blocks.len();
        self.bytes = bytes.len() as u64;
        Ok(())
    }
}

/// Owner-only: a terminal's output is as private as the terminal.
fn private(options: &mut std::fs::OpenOptions) -> &mut std::fs::OpenOptions {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600)
    }
    #[cfg(not(unix))]
    {
        options
    }
}

/// The last blocks within both bounds, oldest first.
pub fn kept(mut blocks: Vec<Block>) -> Vec<Block> {
    if blocks.len() > KEPT_BLOCKS {
        blocks.drain(..blocks.len() - KEPT_BLOCKS);
    }
    let mut bytes: usize = blocks.iter().map(|one| one.output.len()).sum();
    let mut from = 0;
    while bytes > KEPT_BYTES && from + 1 < blocks.len() {
        bytes -= blocks[from].output.len();
        from += 1;
    }
    blocks.drain(..from);
    blocks
}

pub fn encode_block(block: &Block, out: &mut Vec<u8>) {
    let head = &block.head;
    out.push(BLOCK);
    out.extend_from_slice(&head.id.to_le_bytes());
    out.extend_from_slice(&head.started_at.to_le_bytes());
    out.push(u8::from(head.ended_at.is_some()));
    out.extend_from_slice(&head.ended_at.unwrap_or(0).to_le_bytes());
    out.push(u8::from(head.code.is_some()));
    out.extend_from_slice(&head.code.unwrap_or(0).to_le_bytes());
    out.push(
        u8::from(head.interactive)
            | (u8::from(head.truncated) << 1)
            | (u8::from(head.bookmarked) << 2),
    );
    text(out, head.command.as_deref());
    text(out, head.cwd.as_deref());
    let output = &block.output[block.output.len().saturating_sub(MOST_OUTPUT)..];
    out.extend_from_slice(&(output.len() as u32).to_le_bytes());
    out.extend_from_slice(output);
}

fn text(out: &mut Vec<u8>, value: Option<&str>) {
    out.push(u8::from(value.is_some()));
    let bytes = value.unwrap_or("").as_bytes();
    let bytes = &bytes[..bytes.len().min(MOST_TEXT)];
    out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(bytes);
}

/// Every whole record, in order, with bookmarks applied to the blocks before
/// them. A file that is not a journal is none.
pub fn decode(bytes: &[u8]) -> Vec<Block> {
    let Some(mut rest) = bytes.strip_prefix(MAGIC) else {
        return Vec::new();
    };
    let mut blocks: Vec<Block> = Vec::new();
    while let Some((&tag, after)) = rest.split_first() {
        let mut reader = Reader(after);
        let read = match tag {
            BLOCK => reader.block().map(|block| blocks.push(block)),
            MARK => reader.mark().map(|(id, on)| {
                if let Some(block) = blocks.iter_mut().rev().find(|one| one.head.id == id) {
                    block.head.bookmarked = on;
                }
            }),
            _ => None,
        };
        if read.is_none() {
            break;
        }
        rest = reader.0;
    }
    blocks
}

/// The unread end of a record, taken from the front.
struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.0.len() < n {
            return None;
        }
        let (taken, rest) = self.0.split_at(n);
        self.0 = rest;
        Some(taken)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|one| one[0])
    }

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    /// A length the file supplies, refused past `most` before anything is
    /// taken for it.
    fn sized(&mut self, most: usize) -> Option<&'a [u8]> {
        let len = self.u32()? as usize;
        if len > most {
            return None;
        }
        self.take(len)
    }

    fn text(&mut self) -> Option<Option<String>> {
        let present = self.u8()? != 0;
        let bytes = self.sized(MOST_TEXT)?;
        Some(present.then(|| String::from_utf8_lossy(bytes).into_owned()))
    }

    fn block(&mut self) -> Option<Block> {
        let id = self.u64()?;
        let started_at = self.u64()?;
        let ended = (self.u8()? != 0, self.u64()?);
        let code = (
            self.u8()? != 0,
            i32::from_le_bytes(self.take(4)?.try_into().ok()?),
        );
        let flags = self.u8()?;
        let command = self.text()?;
        let cwd = self.text()?;
        let output = self.sized(MOST_OUTPUT)?.to_vec();
        Some(Block {
            head: Head {
                id,
                command,
                cwd,
                started_at,
                ended_at: ended.0.then_some(ended.1),
                code: code.0.then_some(code.1),
                interactive: flags & 1 != 0,
                truncated: flags & 2 != 0,
                bookmarked: flags & 4 != 0,
            },
            output,
        })
    }

    fn mark(&mut self) -> Option<(u64, bool)> {
        Some((self.u64()?, self.u8()? != 0))
    }
}

#[cfg(test)]
#[path = "journal_tests.rs"]
mod tests;
