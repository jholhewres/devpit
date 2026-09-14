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

pub mod agents;
mod foreground;
mod osc;
mod reader;
mod ring;
pub mod shell;
mod stopping;
pub mod usage;

pub use foreground::{all_on as fronts_on, at_a_prompt, looking, on as front_on, Front, Settling};
pub use osc::{Scanner, Told, MOST_CARRIED};
pub use reader::{after_read, AfterRead};
pub use ring::RingBuffer;
pub use stopping::{stop, stop_group, Stopped, GRACE};

use std::io::Write;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use tokio::sync::mpsc;

/// One frame's worth of coalesced output.
pub const FRAME: std::time::Duration = std::time::Duration::from_millis(16);

/// How long to wait before reading again when the pty had nothing to say.
///
/// Short enough not to be felt, long enough that a pty which answers `EAGAIN`
/// in a tight sequence does not become a spinning core.
pub const BETWEEN_READS: std::time::Duration = std::time::Duration::from_millis(4);

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
    /// What the terminal said about itself while producing those bytes.
    ///
    /// A separate channel from the frames because the two have opposite
    /// deadlines: a frame is late if it misses the next paint, and a title is
    /// late if it misses the next glance. Dropping one must never delay the
    /// other. Taken with [`Session::take_told`], because relaying it happens
    /// somewhere other than where the frames are drained.
    told: Option<mpsc::Receiver<Told>>,
    pub counters: Arc<Counters>,
    pub ring: Arc<std::sync::Mutex<RingBuffer>>,
    pid: Option<u32>,
    writer: Option<Box<dyn Write + Send>>,
    master: Option<Box<dyn MasterPty + Send>>,
    killer: Option<Box<dyn ChildKiller + Send + Sync>>,
}

pub struct SessionIo {
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
    /// What to ask about, and to insist to, when this session is ended.
    ///
    /// `None` on a platform that does not report one. See [`stop`].
    pub pid: Option<u32>,
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

    /// Hands over what the terminal says about itself, once.
    ///
    /// Once because there is one stream of it: two takers would each get a
    /// share of the sequences and both would draw a pane half told.
    pub fn take_told(&mut self) -> Option<mpsc::Receiver<Told>> {
        self.told.take()
    }

    /// Hands the io ends to whoever will serve `session.write` while frames drain.
    pub fn take_io(&mut self) -> Option<SessionIo> {
        Some(SessionIo {
            writer: self.writer.take()?,
            master: self.master.take()?,
            killer: self.killer.take()?,
            pid: self.pid,
        })
    }
}

/// Spawns a command on a pty and starts the read path.
pub fn spawn(command: CommandBuilder, size: PtySize) -> Result<Session, PtyError> {
    let pty = NativePtySystem::default()
        .openpty(size)
        .map_err(|e| PtyError::Open(e.to_string()))?;

    let label = format!("{command:?}");
    let child = pty
        .slave
        .spawn_command(command)
        .map_err(|source| PtyError::Spawn {
            command: label,
            source,
        })?;
    let killer = child.clone_killer();
    let pid = child.process_id();

    let reader = pty
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
    // Small, and dropped when full rather than waited on. These are things the
    // screen would like to know; a listener that stopped draining must not be
    // able to stall the bytes of a terminal.
    let (told_tx, told) = mpsc::channel::<Told>(64);
    let counters = Arc::new(Counters::default());
    let ring = Arc::new(std::sync::Mutex::new(RingBuffer::new(RING_BYTES)));

    reader::start(
        reader,
        child,
        Arc::clone(&counters),
        Arc::clone(&ring),
        tx,
        told_tx,
    );

    Ok(Session {
        frames,
        told: Some(told),
        counters,
        ring,
        pid,
        writer: Some(writer),
        master: Some(pty.master),
        killer: Some(killer),
    })
}

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
