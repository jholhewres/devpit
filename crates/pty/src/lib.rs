//! The PTY path: bytes out of a process and onto the screen.
//!
//! ```text
//! PTY ─► reader thread ─► ring buffer ─► coalescer (~16ms) ─► binary frame
//! ```
//!
//! Four decisions, each with a reason:
//!
//! 1. **Raw bytes, not JSON.** Tauri's channel serialises to JSON by default,
//!    and `JSON.parse` costs 0.5–2ms per 5KB payload. A verbose build
//!    saturates that. The frame carries a length and a payload; the JS side
//!    reads it with a `DataView`.
//! 2. **Coalescing on a ~16ms frame.** Never emit per byte. It is what
//!    xterm.js does internally anyway.
//! 3. **The parser runs in Rust.** `ready`/`busy`/`ask` detection has to work
//!    with the window closed, so a notification can fire. If recognition lived
//!    in xterm.js, a session with no open tab would be mute.
//! 4. **Scrollback to a file, screen in memory.** Reattach draws from parsed
//!    state; history search reads the file. Keeping bytes rather than screens
//!    means holding gigabytes in RAM to answer a question about a few
//!    thousand cells.
//!
//! Points 3 and 4 are not built yet; the shape here is what they attach to.

mod ring;

pub use ring::RingBuffer;

use std::io::{Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use tokio::sync::mpsc;

/// One frame's worth of coalesced output.
pub const FRAME: std::time::Duration = std::time::Duration::from_millis(16);

/// How much scrollback is held in memory before the oldest is dropped.
///
/// 2 MiB is roughly ten thousand lines of an 80-column terminal — past what
/// anyone scrolls back through, and small enough that a dozen live sessions
/// stay in a budget a laptop does not notice.
pub const RING_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum PtyError {
    #[error("could not open a pty: {0}")]
    Open(String),

    #[error("could not start {command}: {source}")]
    Spawn {
        command: String,
        #[source]
        source: anyhow::Error,
    },
}

/// What a session has produced, counted rather than estimated.
///
/// The load test reads these. A number that was not measured is absent, not
/// guessed: a zero and an unread figure look identical on screen, and one of
/// them is a lie.
#[derive(Debug, Default)]
pub struct Counters {
    pub bytes: AtomicU64,
    pub frames: AtomicU64,
}

pub struct Session {
    /// Frames of raw bytes, already coalesced.
    pub frames: mpsc::Receiver<Vec<u8>>,
    pub counters: Arc<Counters>,
    pub ring: Arc<std::sync::Mutex<RingBuffer>>,
    writer: Option<Box<dyn Write + Send>>,
    master: Option<Box<dyn MasterPty + Send>>,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
}

pub struct SessionIo {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
}

impl Session {
    pub fn write(&mut self, bytes: &[u8]) -> std::io::Result<()> {
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| std::io::Error::other("this session's writer was taken"))?;
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub fn resize(&self, size: PtySize) -> Result<(), PtyError> {
        let master = self
            .master
            .as_ref()
            .ok_or_else(|| PtyError::Open("this session's master was taken".into()))?;
        master
            .resize(size)
            .map_err(|err| PtyError::Open(err.to_string()))
    }

    /// Hands the io ends to whoever will serve `session.write` while frames drain.
    pub fn take_io(&mut self) -> Option<SessionIo> {
        Some(SessionIo {
            writer: self.writer.take()?,
            master: self.master.take()?,
            killer: self.killer.take()?,
        })
    }
}

/// Spawns a command on a pty and starts the read path.
pub fn spawn(command: CommandBuilder, size: PtySize) -> Result<Session, PtyError> {
    let pty = NativePtySystem::default()
        .openpty(size)
        .map_err(|e| PtyError::Open(e.to_string()))?;

    let label = format!("{command:?}");
    let mut child = pty
        .slave
        .spawn_command(command)
        .map_err(|source| PtyError::Spawn {
            command: label,
            source,
        })?;
    let killer = child.clone_killer();

    let mut reader = pty
        .master
        .try_clone_reader()
        .map_err(|e| PtyError::Open(e.to_string()))?;
    let writer = pty
        .master
        .take_writer()
        .map_err(|e| PtyError::Open(e.to_string()))?;

    // Bounded: an unbounded channel in front of a process that outputs faster
    // than the screen consumes is a memory leak with extra steps. Full means
    // the reader waits, which is backpressure reaching the writing process —
    // exactly what a terminal is supposed to do.
    let (tx, frames) = mpsc::channel::<Vec<u8>>(256);
    let counters = Arc::new(Counters::default());
    let ring = Arc::new(std::sync::Mutex::new(RingBuffer::new(RING_BYTES)));

    let reader_counters = Arc::clone(&counters);
    let reader_ring = Arc::clone(&ring);

    // A blocking thread, not a tokio task: reading a pty fd blocks, and
    // blocking inside the runtime starves every other task on that worker.
    std::thread::spawn(move || {
        let mut buf = [0u8; 64 * 1024];
        let mut pending: Vec<u8> = Vec::with_capacity(64 * 1024);
        let mut last_flush = std::time::Instant::now();

        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let chunk = &buf[..n];
                    reader_counters.bytes.fetch_add(n as u64, Ordering::Relaxed);
                    if let Ok(mut ring) = reader_ring.lock() {
                        ring.write(chunk);
                    }
                    pending.extend_from_slice(chunk);
                }
                Err(_) => break,
            }

            // Coalesce: hand over a frame's worth at a time rather than every
            // read. Under a flood this turns tens of thousands of tiny
            // messages into about sixty per second.
            if last_flush.elapsed() >= FRAME && !pending.is_empty() {
                reader_counters.frames.fetch_add(1, Ordering::Relaxed);
                if tx.blocking_send(std::mem::take(&mut pending)).is_err() {
                    break;
                }
                pending = Vec::with_capacity(64 * 1024);
                last_flush = std::time::Instant::now();
            }
        }

        if !pending.is_empty() {
            reader_counters.frames.fetch_add(1, Ordering::Relaxed);
            let _ = tx.blocking_send(pending);
        }

        let _ = child.wait();
    });

    Ok(Session {
        frames,
        counters,
        ring,
        writer: Some(writer),
        master: Some(pty.master),
        killer: Some(killer),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size() -> PtySize {
        PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        }
    }

    #[tokio::test]
    async fn a_command_output_arrives_as_frames() {
        let mut cmd = CommandBuilder::new("echo");
        cmd.arg("devpit");

        let mut session = spawn(cmd, size()).expect("spawn");

        let mut seen = Vec::new();
        while let Some(frame) = session.frames.recv().await {
            seen.extend_from_slice(&frame);
        }

        assert!(
            String::from_utf8_lossy(&seen).contains("devpit"),
            "output did not reach the frames: {:?}",
            String::from_utf8_lossy(&seen)
        );
        assert!(session.counters.bytes.load(Ordering::Relaxed) > 0);
    }

    #[tokio::test]
    async fn bytes_written_to_the_pty_come_back_in_frames() {
        let cmd = CommandBuilder::new("cat");
        let mut session = spawn(cmd, size()).expect("spawn");

        session.write(b"devpit-stdin\n").expect("write");
        // Closing stdin is what makes `cat` exit. Dropping the writer is that close.
        let io = session.take_io().expect("io");
        drop(io.writer);

        let mut seen = Vec::new();
        while let Some(frame) = session.frames.recv().await {
            seen.extend_from_slice(&frame);
        }
        assert!(
            String::from_utf8_lossy(&seen).contains("devpit-stdin"),
            "stdin did not echo: {:?}",
            String::from_utf8_lossy(&seen)
        );
    }

    /// The point of coalescing: a flood must not become one message per read.
    #[tokio::test]
    async fn a_flood_is_coalesced_into_far_fewer_frames_than_reads() {
        let mut cmd = CommandBuilder::new("sh");
        cmd.arg("-c");
        cmd.arg("yes devpit | head -c 2000000");

        let mut session = spawn(cmd, size()).expect("spawn");

        let mut bytes = 0usize;
        while let Some(frame) = session.frames.recv().await {
            bytes += frame.len();
        }

        let frames = session.counters.frames.load(Ordering::Relaxed);
        assert!(bytes >= 2_000_000, "only {bytes} bytes arrived");
        assert!(
            frames < 400,
            "{frames} frames for {bytes} bytes — coalescing is not working"
        );
    }
}
