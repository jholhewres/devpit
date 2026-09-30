//! Following a pane's copy in a regular file, where there is no fifo.
//!
//! psmux, the tmux of Windows, opens the file `pipe-pane` names by itself and
//! writes into it for as long as the pane is piped. Nothing blocks a reader
//! there, so this looks: read to the end, wait a little, read again. A file
//! that got shorter was opened anew by an arming, and is read from its start.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How long a quiet pane waits between looks: short enough not to be felt,
/// and cheap for a pane that says nothing for hours.
pub(crate) const BETWEEN_LOOKS: Duration = Duration::from_millis(40);

/// Empties the file a pane is about to be copied into, and says where reading
/// starts. A file psmux still holds from an earlier arming may refuse to be
/// emptied; it is then read from its end, so old output is not heard twice.
pub(crate) fn emptied(path: &Path) -> Option<u64> {
    match File::create(path) {
        Ok(_) => Some(0),
        Err(_) => std::fs::metadata(path).ok().map(|meta| meta.len()),
    }
}

/// One copy being read, and how far.
pub(crate) struct Tail {
    file: File,
    at: u64,
}

impl Tail {
    pub(crate) fn open(path: &Path, at: u64) -> std::io::Result<Self> {
        Ok(Self {
            file: File::open(path)?,
            at,
        })
    }

    /// What was written since the last read, at most a buffer of it; `0`
    /// when there is nothing new.
    pub(crate) fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let len = self.file.metadata()?.len();
        if len < self.at {
            self.at = 0;
        }
        if len == self.at {
            return Ok(0);
        }
        self.file.seek(SeekFrom::Start(self.at))?;
        let read = self.file.read(buffer)?;
        self.at += read as u64;
        Ok(read)
    }
}

/// Reads a copy for as long as `listening` holds, handing each chunk to
/// `heard`, and asks `rotate` once every time the file grows past `ceiling`.
///
/// Memory stays one buffer however large the file gets; the ceiling is about
/// the disk. The file cannot be cut from here while psmux writes it, so
/// `rotate` arms the pipe again, which opens it anew and empty.
pub(crate) fn follow(
    mut tail: Tail,
    listening: &AtomicBool,
    ceiling: u64,
    mut heard: impl FnMut(&[u8]),
    mut rotate: impl FnMut(),
) {
    let mut buffer = [0u8; 8192];
    let mut asked = false;
    while listening.load(Ordering::Relaxed) {
        match tail.read(&mut buffer) {
            Ok(0) => {
                // At the end, so what a rotation loses is only what arrives
                // between here and psmux reopening the file.
                if tail.at > ceiling && !asked {
                    rotate();
                }
                asked = tail.at > ceiling;
                std::thread::sleep(BETWEEN_LOOKS);
            }
            Ok(read) => heard(&buffer[..read]),
            Err(_) => break,
        }
    }
}

#[cfg(test)]
#[path = "tap_file_tests.rs"]
mod tests;
