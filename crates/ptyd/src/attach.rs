//! `devpit ptyd-attach <leaf>`: one pane the daemon holds, on this terminal.
//!
//! What makes a ptyd pane reachable over ssh: run inside a tmux session, the
//! person attaches to that. Keys go in as `Input`, the tty's size as `Resize`
//! when it changes, and the pane's output is written out as it comes.
//!
//! The daemon keeps one count of unacknowledged output per pane, shared by
//! every attached client. This one attaches as a mirror ([`frame::MIRROR`]):
//! beside a desktop window its acks are dropped, so the pane runs at the
//! window's pace. Alone, it acks all it was sent ([`owed`]), and when it goes
//! quiet whatever the count still holds, as the desktop's tap does: an ack can
//! land before the daemon counts the output it answers, and the leftover
//! would otherwise creep up until the pane stops.

use std::io::{self, Read, Write};
use std::net::Shutdown;
use std::os::unix::io::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::frame::{self, Frame, Kind};

/// Quiet this long after output, the count the daemon holds is settled.
const IDLE: Duration = Duration::from_millis(250);

/// Keys read from the terminal at most at a time, well under a frame.
const KEYS: usize = 16 * 1024;

/// Why attaching ended.
#[derive(Debug, PartialEq, Eq)]
pub enum Ended {
    /// The terminal closed, or its output did: the pane runs on.
    Left,
    /// The pane ended, with its exit code when it reported one.
    Exited(Option<i32>),
    /// The daemon refused: no such pane, most likely.
    Refused(String),
    /// The daemon went.
    Lost,
}

/// What a client owes the daemon for `one`: the output it was sent. A replay
/// is history the daemon never counted.
pub fn owed(one: &Frame) -> Option<u32> {
    (one.kind() == Some(Kind::Output)).then(|| u32::try_from(one.bytes.len()).unwrap_or(u32::MAX))
}

/// Whether to tell the pane this terminal's size at once. Under tmux, no: a
/// session made detached is 80x24 until someone attaches, and the pane is the
/// desktop's size until then; tmux resizing it on attach is the signal.
pub fn sized_at_start(under_tmux: bool) -> bool {
    !under_tmux
}

/// Attaches pane `pane` of the daemon at `socket` to this terminal until one
/// of them ends. The exit code: the pane's own when it ended.
pub fn run(socket: &Path, pane: u32) -> i32 {
    let stream = match crate::socket::connect(socket) {
        Ok(stream) => stream,
        Err(err) => {
            eprintln!("devpit ptyd-attach: devpit-ptyd did not answer: {err}");
            return 1;
        }
    };
    let tty = io::stdin().as_raw_fd();
    let size = tty_size(tty)
        .filter(|_| sized_at_start(std::env::var_os("TMUX").is_some_and(|tmux| !tmux.is_empty())));
    let sizes = sizes_on_winch(tty);
    let raw = Raw::on(tty);
    let ended = serve(stream, pane, size, io::stdin(), &mut io::stdout(), sizes);
    // Before anything is said, so it is said on a terminal that is itself.
    drop(raw);
    match ended {
        Ok(Ended::Left) => 0,
        Ok(Ended::Exited(code)) => code.unwrap_or(0),
        Ok(Ended::Refused(why)) => {
            eprintln!("devpit ptyd-attach: {why}");
            1
        }
        Ok(Ended::Lost) => {
            eprintln!("devpit ptyd-attach: devpit-ptyd went away");
            1
        }
        Err(err) => {
            eprintln!("devpit ptyd-attach: {err}");
            1
        }
    }
}

/// Pane `pane` over `stream`: `input` typed into it, its output to `output`,
/// each size from `sizes` given to it. Detached before it returns.
pub fn serve(
    stream: UnixStream,
    pane: u32,
    size: Option<(u16, u16)>,
    input: impl Read + Send + 'static,
    output: &mut impl Write,
    sizes: Receiver<(u16, u16)>,
) -> io::Result<Ended> {
    let mut reading = stream.try_clone()?;
    let writing = Arc::new(Mutex::new(stream));
    send(
        &writing,
        Frame::new(pane, Kind::Attach, vec![frame::MIRROR]),
    )?;
    if let Some((rows, cols)) = size {
        send(&writing, resize(pane, rows, cols))?;
    }
    let left = Arc::new(AtomicBool::new(false));
    {
        let (writing, left) = (Arc::clone(&writing), Arc::clone(&left));
        std::thread::spawn(move || type_in(input, pane, &writing, &left));
    }
    {
        let writing = Arc::clone(&writing);
        std::thread::spawn(move || {
            for (rows, cols) in sizes {
                if send(&writing, resize(pane, rows, cols)).is_err() {
                    return;
                }
            }
        });
    }
    let mut unsettled = false;
    let ended = loop {
        if unsettled && !readable_within(&reading, IDLE) {
            unsettled = false;
            let _ = ack(&writing, pane, u32::MAX);
            continue;
        }
        let one = match frame::read(&mut reading) {
            Ok(Some(one)) if one.pane == pane => one,
            Ok(Some(_)) => continue,
            Ok(None) | Err(_) if left.load(Ordering::Acquire) => break Ended::Left,
            Ok(None) | Err(_) => break Ended::Lost,
        };
        match one.kind() {
            Some(Kind::Output | Kind::Replay) => {
                let wrote = output.write_all(&one.bytes).and_then(|()| output.flush());
                if let Some(bytes) = owed(&one) {
                    let _ = ack(&writing, pane, bytes);
                    unsettled = true;
                }
                if wrote.is_err() {
                    break Ended::Left;
                }
            }
            Some(Kind::Exited) => break Ended::Exited(exit_code(&one.bytes)),
            Some(Kind::Refused) => {
                break Ended::Refused(String::from_utf8_lossy(&one.bytes).into_owned())
            }
            _ => {}
        }
    };
    let _ = send(&writing, Frame::new(pane, Kind::Detach, Vec::new()));
    if let Ok(stream) = writing.lock() {
        let _ = stream.shutdown(Shutdown::Both);
    }
    Ok(ended)
}

/// Keys into the pane until the terminal closes; then the reading side is
/// woken with the connection's end.
fn type_in(mut input: impl Read, pane: u32, writing: &Mutex<UnixStream>, left: &AtomicBool) {
    let mut buf = vec![0u8; KEYS];
    loop {
        match input.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if send(writing, Frame::new(pane, Kind::Input, buf[..n].to_vec())).is_err() {
                    return;
                }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }
    left.store(true, Ordering::Release);
    if let Ok(stream) = writing.lock() {
        let _ = stream.shutdown(Shutdown::Both);
    }
}

fn send(writing: &Mutex<UnixStream>, one: Frame) -> io::Result<()> {
    let mut stream = writing
        .lock()
        .map_err(|_| io::Error::other("ptyd-attach stream lock"))?;
    frame::write(&mut *stream, &one)
}

fn ack(writing: &Mutex<UnixStream>, pane: u32, bytes: u32) -> io::Result<()> {
    send(
        writing,
        Frame::new(pane, Kind::Ack, bytes.to_le_bytes().to_vec()),
    )
}

fn resize(pane: u32, rows: u16, cols: u16) -> Frame {
    let mut bytes = rows.to_le_bytes().to_vec();
    bytes.extend_from_slice(&cols.to_le_bytes());
    Frame::new(pane, Kind::Resize, bytes)
}

fn exit_code(bytes: &[u8]) -> Option<i32> {
    let b = bytes.get(..4)?;
    Some(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Whether `stream` has something to read within `wait`. An error says yes:
/// the read that follows is what reports it.
fn readable_within(stream: &UnixStream, wait: Duration) -> bool {
    let mut poll = libc::pollfd {
        fd: stream.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let millis = i32::try_from(wait.as_millis()).unwrap_or(i32::MAX);
    // SAFETY: one pollfd on this stack, for an fd `stream` keeps open.
    unsafe { libc::poll(&mut poll, 1, millis) != 0 }
}

/// The terminal at `fd`'s size as rows and columns, when it is one.
fn tty_size(fd: i32) -> Option<(u16, u16)> {
    // SAFETY: a zeroed winsize is a valid one, filled in by the ioctl.
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    // SAFETY: TIOCGWINSZ writes one winsize into `size`, which outlives it.
    let asked = unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, &mut size) };
    (asked == 0 && size.ws_row > 0 && size.ws_col > 0).then_some((size.ws_row, size.ws_col))
}

/// The write end of the pipe SIGWINCH is told through: a handler may do
/// almost nothing, and writing one byte is something it may.
static WINCH: AtomicI32 = AtomicI32::new(-1);

extern "C" fn on_winch(_: libc::c_int) {
    let fd = WINCH.load(Ordering::Relaxed);
    if fd >= 0 {
        // SAFETY: write(2) is async-signal-safe; one byte from a static.
        unsafe { libc::write(fd, [1u8].as_ptr().cast(), 1) };
    }
}

/// The terminal at `fd`'s size each time it changes.
fn sizes_on_winch(fd: i32) -> Receiver<(u16, u16)> {
    let (sized, sizes) = channel();
    let mut ends = [-1; 2];
    // SAFETY: `ends` is two fds' room; pipe(2) fills both or neither.
    if unsafe { libc::pipe(ends.as_mut_ptr()) } != 0 {
        return sizes;
    }
    let [wake, told] = ends;
    // SAFETY: plain fcntl on the pipe just made; the handler must not block
    // on a full pipe, and a dropped wake-up is one the next one covers.
    unsafe { libc::fcntl(told, libc::F_SETFL, libc::O_NONBLOCK) };
    WINCH.store(told, Ordering::Relaxed);
    // SAFETY: a zeroed sigaction is empty; the handler only writes a byte.
    unsafe {
        let mut action: libc::sigaction = std::mem::zeroed();
        action.sa_sigaction = on_winch as extern "C" fn(libc::c_int) as libc::sighandler_t;
        action.sa_flags = libc::SA_RESTART;
        libc::sigemptyset(&mut action.sa_mask);
        libc::sigaction(libc::SIGWINCH, &action, std::ptr::null_mut());
    }
    std::thread::spawn(move || {
        let mut byte = [0u8; 1];
        loop {
            // SAFETY: one byte into this frame's buffer from the pipe's end.
            let n = unsafe { libc::read(wake, byte.as_mut_ptr().cast(), 1) };
            if n <= 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                return;
            }
            if let Some(size) = tty_size(fd) {
                if sized.send(size).is_err() {
                    return;
                }
            }
        }
    });
    sizes
}

/// The terminal in raw mode for as long as this lives: every key goes to the
/// pane as typed, Ctrl+C included. Put back as it was when dropped, on an
/// error's way out as on the usual one.
struct Raw {
    fd: i32,
    was: libc::termios,
}

impl Raw {
    fn on(fd: i32) -> Option<Raw> {
        // SAFETY: isatty only asks.
        if unsafe { libc::isatty(fd) } != 1 {
            return None;
        }
        // SAFETY: a zeroed termios is overwritten whole by tcgetattr.
        let mut was: libc::termios = unsafe { std::mem::zeroed() };
        // SAFETY: tcgetattr fills `was`, which outlives the call.
        if unsafe { libc::tcgetattr(fd, &mut was) } != 0 {
            return None;
        }
        let mut raw = was;
        // SAFETY: both on this stack; cfmakeraw only edits `raw`.
        unsafe {
            libc::cfmakeraw(&mut raw);
            if libc::tcsetattr(fd, libc::TCSANOW, &raw) != 0 {
                return None;
            }
        }
        Some(Raw { fd, was })
    }
}

impl Drop for Raw {
    fn drop(&mut self) {
        // SAFETY: the settings read from this same fd before.
        unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.was) };
    }
}

#[cfg(test)]
#[path = "attach_tests.rs"]
mod tests;
