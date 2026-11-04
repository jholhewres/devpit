//! The wire: `[u32 len][u32 pane][u8 kind][bytes]`, little-endian, `len`
//! counting `pane`, `kind` and the bytes.
//!
//! Binary on purpose: a pane's output is bytes, and base64 inside JSON costs a
//! third more on the wire and a parse per chunk on both ends.

use std::io::{self, Read, Write};

/// The largest frame either side accepts. A slice is ~45 KB; this leaves room
/// for a replay and refuses a length that is a corrupted stream, not a frame.
pub const MOST_FRAME: u32 = 1024 * 1024;

/// What a frame is. One byte, so an unknown kind is skipped, not fatal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Kind {
    /// Client → daemon, first frame: the protocol version it speaks (u32).
    Hello = 1,
    /// Daemon → client: the version it speaks, answering `Hello`.
    Welcome = 2,
    /// Client → daemon: start a pane. Bytes: `rows u16, cols u16`, then the
    /// cwd and the argv, each NUL-terminated.
    Spawn = 3,
    /// Client → daemon: bytes for the pane's pty.
    Input = 4,
    /// Client → daemon: `rows u16, cols u16`.
    Resize = 5,
    /// Client → daemon: end the pane.
    Kill = 6,
    /// Client → daemon: send the pane from now on, after its replay.
    Attach = 7,
    /// Client → daemon: stop sending the pane; it keeps running.
    Detach = 8,
    /// Client → daemon: bytes consumed (u32), so the pty may read on.
    Ack = 9,
    /// Client → daemon: `have u64`, how much of the history it already holds.
    History = 10,
    /// Daemon → client: bytes the pane printed.
    Output = 11,
    /// Daemon → client: replay before live output (modes, then the tail).
    Replay = 12,
    /// Daemon → client: older history, answering `History`. Bytes: `left u64`
    /// then the chunk.
    Older = 13,
    /// Daemon → client: the pane's cwd changed (UTF-8 path).
    Cwd = 14,
    /// Daemon → client: the foreground program changed (UTF-8 name).
    Program = 15,
    /// Daemon → client: the pane ended. Bytes: exit code as i32, or none.
    Exited = 16,
    /// Daemon → client: a request was refused (UTF-8 reason).
    Refused = 17,
    /// Client → daemon: send every pane's `Program`, `Cwd` and `Exited`, now
    /// and from here on, without its output. The pane id is ignored.
    Watch = 18,
}

impl Kind {
    pub fn of(byte: u8) -> Option<Kind> {
        use Kind::*;
        Some(match byte {
            1 => Hello,
            2 => Welcome,
            3 => Spawn,
            4 => Input,
            5 => Resize,
            6 => Kill,
            7 => Attach,
            8 => Detach,
            9 => Ack,
            10 => History,
            11 => Output,
            12 => Replay,
            13 => Older,
            14 => Cwd,
            15 => Program,
            16 => Exited,
            17 => Refused,
            18 => Watch,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub pane: u32,
    /// The raw kind byte; [`Kind::of`] names it.
    pub kind: u8,
    pub bytes: Vec<u8>,
}

impl Frame {
    pub fn new(pane: u32, kind: Kind, bytes: impl Into<Vec<u8>>) -> Frame {
        Frame {
            pane,
            kind: kind as u8,
            bytes: bytes.into(),
        }
    }

    pub fn kind(&self) -> Option<Kind> {
        Kind::of(self.kind)
    }

    pub fn encode(&self) -> Vec<u8> {
        let len = 5 + self.bytes.len() as u32;
        let mut out = Vec::with_capacity(4 + len as usize);
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&self.pane.to_le_bytes());
        out.push(self.kind);
        out.extend_from_slice(&self.bytes);
        out
    }
}

pub fn write(to: &mut impl Write, frame: &Frame) -> io::Result<()> {
    to.write_all(&frame.encode())
}

/// One frame, or `None` at a clean end of stream. A length under the header
/// or over [`MOST_FRAME`] is an error: the stream is out of step.
pub fn read(from: &mut impl Read) -> io::Result<Option<Frame>> {
    let mut len = [0u8; 4];
    match from.read_exact(&mut len) {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err),
    }
    let len = u32::from_le_bytes(len);
    if !(5..=MOST_FRAME).contains(&len) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("a frame of {len} bytes"),
        ));
    }
    let mut body = vec![0u8; len as usize];
    from.read_exact(&mut body)?;
    let pane = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
    let kind = body[4];
    body.drain(..5);
    Ok(Some(Frame {
        pane,
        kind,
        bytes: body,
    }))
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
