//! One terminal the daemon holds: its pty, its history and modes, who is
//! watching, and the thread that reads it.

use std::io::{Read, Write};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};

use crate::client::Client;
use crate::frame::{Frame, Kind};
use crate::history::History;
use crate::modes::Modes;

/// One slice of output, at most: what is drained from the pty in one go.
pub const SLICE: usize = 45 * 1024;
/// Sent and not yet acknowledged, past which the pty is not read: the program
/// then blocks on a full tty, which is what makes Ctrl+C answer mid-flood.
pub const MOST_UNACKED: usize = 1024 * 1024;
/// Quiet this long after output before the foreground program is looked at.
pub const SETTLE: Duration = Duration::from_millis(250);
/// History kept per pane.
pub const KEPT: usize = devpit_pty::RING_BYTES;

pub struct Pane {
    pub id: u32,
    state: Mutex<State>,
    flow: Mutex<usize>,
    acked: Condvar,
    quiet: (Mutex<Option<Instant>>, Condvar),
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
}

struct State {
    history: History,
    modes: Modes,
    watchers: Vec<Arc<Client>>,
    cwd: Option<String>,
    program: Option<String>,
    ended: bool,
    /// The exit code, once ended and when the process reported one.
    code: Option<i32>,
}

/// What a `Spawn` frame asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asked {
    pub rows: u16,
    pub cols: u16,
    pub cwd: String,
    pub argv: Vec<String>,
}

impl Asked {
    /// `rows u16, cols u16`, then the cwd and each argv word NUL-terminated.
    pub fn parse(bytes: &[u8]) -> Option<Asked> {
        let rows = u16::from_le_bytes([*bytes.first()?, *bytes.get(1)?]);
        let cols = u16::from_le_bytes([*bytes.get(2)?, *bytes.get(3)?]);
        let mut words = bytes[4..]
            .split(|b| *b == 0)
            .map(|word| String::from_utf8_lossy(word).into_owned());
        let cwd = words.next()?;
        let argv: Vec<String> = words.filter(|word| !word.is_empty()).collect();
        (!argv.is_empty() && rows > 0 && cols > 0).then_some(Asked {
            rows,
            cols,
            cwd,
            argv,
        })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.rows.to_le_bytes());
        out.extend_from_slice(&self.cols.to_le_bytes());
        for word in std::iter::once(&self.cwd).chain(&self.argv) {
            out.extend_from_slice(word.as_bytes());
            out.push(0);
        }
        out
    }
}

pub fn size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

impl Pane {
    /// Starts `asked` on a pty and the threads that read and watch it.
    /// `ended` is told once the pane is gone, so the daemon can forget it.
    pub fn spawn(
        id: u32,
        asked: &Asked,
        ended: impl FnOnce(u32) + Send + 'static,
    ) -> Result<Arc<Pane>, String> {
        let pty = NativePtySystem::default()
            .openpty(size(asked.rows, asked.cols))
            .map_err(|err| err.to_string())?;
        let mut command = CommandBuilder::new(&asked.argv[0]);
        command.args(&asked.argv[1..]);
        if std::path::Path::new(&asked.cwd).is_dir() {
            command.cwd(&asked.cwd);
        }
        devpit_pty::host_env::scrub_pty(&mut command);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        let child = pty
            .slave
            .spawn_command(command)
            .map_err(|err| format!("could not start {}: {err}", asked.argv[0]))?;
        drop(pty.slave);
        let reader = pty
            .master
            .try_clone_reader()
            .map_err(|err| err.to_string())?;
        let writer = pty.master.take_writer().map_err(|err| err.to_string())?;
        let pane = Arc::new(Pane {
            id,
            state: Mutex::new(State {
                history: History::new(KEPT),
                modes: Modes::new(),
                watchers: Vec::new(),
                cwd: None,
                program: None,
                ended: false,
                code: None,
            }),
            flow: Mutex::new(0),
            acked: Condvar::new(),
            quiet: (Mutex::new(None), Condvar::new()),
            writer: Mutex::new(writer),
            killer: Mutex::new(child.clone_killer()),
            master: Mutex::new(pty.master),
        });
        let reading = Arc::clone(&pane);
        std::thread::spawn(move || {
            reading.read(reader, child);
            ended(reading.id);
        });
        let watching = Arc::clone(&pane);
        std::thread::spawn(move || watching.watch_program());
        Ok(pane)
    }

    pub fn write(&self, bytes: &[u8]) -> std::io::Result<()> {
        let mut writer = self.writer.lock().map_err(|_| poisoned())?;
        writer.write_all(bytes)?;
        writer.flush()
    }

    pub fn resize(&self, rows: u16, cols: u16) -> Result<(), String> {
        let master = self.master.lock().map_err(|_| "master lock".to_owned())?;
        master
            .resize(size(rows, cols))
            .map_err(|err| err.to_string())
    }

    pub fn kill(&self) {
        if let Ok(mut killer) = self.killer.lock() {
            let _ = killer.kill();
        }
    }

    /// Puts `client` back into the pane's modes, sends the last of its
    /// history, then sends it everything from here on. Under the state lock,
    /// so no slice falls between the replay and the first live one.
    pub fn attach(&self, client: &Arc<Client>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let restore = state.modes.restore();
        if !restore.is_empty() {
            client.send(&Frame::new(self.id, Kind::Replay, restore));
        }
        let tail = state.history.tail(crate::history::REPLAYED);
        if !tail.is_empty() {
            client.send(&Frame::new(self.id, Kind::Replay, tail));
        }
        if let Some(cwd) = state.cwd.clone() {
            client.send(&Frame::new(self.id, Kind::Cwd, cwd));
        }
        state.watchers.retain(|one| !Arc::ptr_eq(one, client));
        if state.ended {
            client.send(&exited(self.id, state.code));
        } else {
            state.watchers.push(Arc::clone(client));
        }
    }

    pub fn detach(&self, client: &Arc<Client>) {
        if let Ok(mut state) = self.state.lock() {
            state.watchers.retain(|one| !Arc::ptr_eq(one, client));
            if state.watchers.is_empty() {
                self.release_flow();
            }
        }
    }

    /// The page before the last `have` bytes: `left u64` then the bytes.
    pub fn older(&self, have: usize) -> Vec<u8> {
        let Ok(state) = self.state.lock() else {
            return Vec::new();
        };
        let (page, left) = state.history.older(have);
        let mut out = (left as u64).to_le_bytes().to_vec();
        out.extend(page);
        out
    }

    pub fn ack(&self, bytes: usize) {
        if let Ok(mut unacked) = self.flow.lock() {
            *unacked = unacked.saturating_sub(bytes);
            self.acked.notify_all();
        }
    }

    /// Nobody left to ack: nothing should hold the pty.
    fn release_flow(&self) {
        if let Ok(mut unacked) = self.flow.lock() {
            *unacked = 0;
            self.acked.notify_all();
        }
    }

    pub fn unacked(&self) -> usize {
        self.flow.lock().map(|unacked| *unacked).unwrap_or(0)
    }

    pub fn cwd(&self) -> Option<String> {
        self.state.lock().ok()?.cwd.clone()
    }

    pub fn program(&self) -> Option<String> {
        self.state.lock().ok()?.program.clone()
    }

    fn read(
        &self,
        mut reader: Box<dyn Read + Send>,
        mut child: Box<dyn portable_pty::Child + Send + Sync>,
    ) {
        let raw = self
            .master
            .lock()
            .ok()
            .and_then(|master| master.as_raw_fd());
        let mut buf = vec![0u8; SLICE];
        let mut scanner = devpit_pty::Scanner::new();
        loop {
            self.wait_for_acks();
            let mut filled = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(error) => {
                    let exited = matches!(child.try_wait(), Ok(Some(_)));
                    match devpit_pty::after_read(&error, exited) {
                        devpit_pty::AfterRead::Again => {
                            std::thread::sleep(devpit_pty::BETWEEN_READS);
                            continue;
                        }
                        _ => break,
                    }
                }
            };
            // Drained to EAGAIN, up to a slice: a burst goes out as one frame.
            while filled < SLICE && raw.is_some_and(readable_now) {
                match reader.read(&mut buf[filled..]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => filled += n,
                }
            }
            self.heard(&buf[..filled], &mut scanner);
        }
        let code = child.wait().ok().map(|status| status.exit_code() as i32);
        self.ended(code);
    }

    fn wait_for_acks(&self) {
        let Ok(mut unacked) = self.flow.lock() else {
            return;
        };
        while *unacked > MOST_UNACKED {
            match self.acked.wait_timeout(unacked, Duration::from_secs(1)) {
                Ok((guard, _)) => unacked = guard,
                Err(_) => return,
            }
        }
    }

    fn heard(&self, slice: &[u8], scanner: &mut devpit_pty::Scanner) {
        let mut moved = None;
        scanner.scan(slice, |told| {
            if let devpit_pty::Told::Cwd(cwd) = told {
                moved = Some(cwd);
            }
        });
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.history.write(slice);
        state.modes.feed(slice);
        let output = Frame::new(self.id, Kind::Output, slice.to_vec());
        state.watchers.retain(|client| client.send(&output));
        if !state.watchers.is_empty() {
            if let Ok(mut unacked) = self.flow.lock() {
                *unacked += slice.len();
            }
        }
        if let Some(cwd) = moved.filter(|cwd| state.cwd.as_ref() != Some(cwd)) {
            let told = Frame::new(self.id, Kind::Cwd, cwd.clone());
            state.watchers.retain(|client| client.send(&told));
            state.cwd = Some(cwd);
        }
        drop(state);
        let (when, wake) = &self.quiet;
        if let Ok(mut last) = when.lock() {
            *last = Some(Instant::now());
            wake.notify_one();
        }
    }

    fn ended(&self, code: Option<i32>) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.ended = true;
        state.code = code;
        let told = exited(self.id, code);
        for client in state.watchers.drain(..) {
            client.send(&told);
        }
        drop(state);
        self.release_flow();
        let (when, wake) = &self.quiet;
        if let Ok(mut last) = when.lock() {
            *last = None;
            wake.notify_one();
        }
    }

    /// Looks at the foreground program once output has been quiet for
    /// [`SETTLE`], and says so when it changed. Ends with the pane.
    fn watch_program(&self) {
        let (when, wake) = &self.quiet;
        loop {
            let Ok(mut last) = when.lock() else { return };
            loop {
                if self.state.lock().map(|state| state.ended).unwrap_or(true) {
                    return;
                }
                match *last {
                    Some(at) if at.elapsed() >= SETTLE => break,
                    Some(at) => {
                        let wait = SETTLE - at.elapsed();
                        last = match wake.wait_timeout(last, wait) {
                            Ok((guard, _)) => guard,
                            Err(_) => return,
                        };
                    }
                    None => {
                        last = match wake.wait_timeout(last, Duration::from_secs(5)) {
                            Ok((guard, _)) => guard,
                            Err(_) => return,
                        };
                    }
                }
            }
            *last = None;
            drop(last);
            self.look_at_program();
        }
    }

    fn look_at_program(&self) {
        let leader = self
            .master
            .lock()
            .ok()
            .and_then(|master| master.process_group_leader());
        let Some(name) = leader.and_then(program_of) else {
            return;
        };
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        if state.program.as_deref() == Some(name.as_str()) {
            return;
        }
        let told = Frame::new(self.id, Kind::Program, name.clone());
        state.watchers.retain(|client| client.send(&told));
        state.program = Some(name);
    }
}

/// `Exited`, with the code as i32 when there is one.
fn exited(pane: u32, code: Option<i32>) -> Frame {
    let bytes = code
        .map(|code| code.to_le_bytes().to_vec())
        .unwrap_or_default();
    Frame::new(pane, Kind::Exited, bytes)
}

fn poisoned() -> std::io::Error {
    std::io::Error::other("a lock was poisoned")
}

/// Whether `fd` has bytes to read right now, without waiting.
fn readable_now(fd: i32) -> bool {
    let mut poll = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one pollfd on this stack, for an fd the pane keeps open.
    unsafe { libc::poll(&mut poll, 1, 0) > 0 && poll.revents & libc::POLLIN != 0 }
}

/// The name of process `pid`, as `ps` would show it.
fn program_of(pid: i32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let comm = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?;
        Some(comm.trim().to_owned()).filter(|name| !name.is_empty())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let out = std::process::Command::new("ps")
            .args(["-o", "comm=", "-p", &pid.to_string()])
            .output()
            .ok()?;
        let name = String::from_utf8_lossy(&out.stdout).trim().to_owned();
        let base = name.rsplit('/').next().unwrap_or_default().to_owned();
        Some(base).filter(|name| !name.is_empty())
    }
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
