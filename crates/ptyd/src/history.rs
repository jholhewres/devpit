//! A pane's history: the last 2 MiB it printed, what a late client is shown
//! first, and older pages on demand.

use devpit_pty::RingBuffer;

/// What a client is sent on attach, after the modes.
pub const REPLAYED: usize = 256 * 1024;
/// One page of older history, asked for with `History`.
pub const PAGE: usize = 256 * 1024;

pub struct History {
    ring: RingBuffer,
}

impl History {
    pub fn new(capacity: usize) -> History {
        History {
            ring: RingBuffer::new(capacity),
        }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        self.ring.write(bytes);
    }

    pub fn len(&self) -> usize {
        self.ring.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ring.is_empty()
    }

    /// The last `most` bytes, started where a terminal can begin drawing.
    pub fn tail(&self, most: usize) -> Vec<u8> {
        let all = self.ring.contents();
        let from = all.len().saturating_sub(most);
        let cut = safe_start(&all[from..]);
        all[from + cut..].to_vec()
    }

    /// The page before the last `have` bytes, and how much is left before it.
    /// Cut at the same kind of point as [`History::tail`], so pages join up.
    pub fn older(&self, have: usize) -> (Vec<u8>, usize) {
        let all = self.ring.contents();
        let end = all.len().saturating_sub(have);
        let from = end.saturating_sub(PAGE);
        let cut = if from == 0 {
            0
        } else {
            safe_start(&all[from..end])
        };
        (all[from + cut..end].to_vec(), from + cut)
    }
}

/// Where in `bytes` a terminal can start: at an ESC, or after a newline,
/// whichever comes first. Starting mid-sequence draws `[31m` as text; starting
/// mid-character draws a replacement glyph. With neither, the first whole
/// UTF-8 character.
pub fn safe_start(bytes: &[u8]) -> usize {
    let esc = bytes.iter().position(|b| *b == 0x1b);
    let line = bytes.iter().position(|b| *b == b'\n').map(|at| at + 1);
    match (esc, line) {
        (Some(esc), Some(line)) => esc.min(line),
        (Some(at), None) | (None, Some(at)) => at,
        (None, None) => bytes
            .iter()
            .position(|byte| byte & 0xc0 != 0x80)
            .unwrap_or(bytes.len()),
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
