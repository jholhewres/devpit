//! Listening to a pane nobody is looking at.
//!
//! [`devpit_pty::osc`] says why the parser is in Rust rather than in the
//! webview:
//!
//! > *recognition has to work with the window closed. A parser living in the
//! > webview is a parser that is asleep exactly when a notification would be
//! > worth sending.*
//!
//! It was half true. The parser is in Rust, and until this it only ran while a
//! pane was attached — so closing the tab stopped the watching, which is the
//! moment the watching is worth most.
//!
//! tmux keeps piping a pane's output with no client attached, so the tap arms
//! `pipe-pane` per window and reads the copy. Two failure modes were measured
//! before relying on it: a reader that dies leaves the pane running, and a
//! reader that stalls does not stall the pane — two hundred thousand lines
//! with nobody draining, and the pane still answered.
//!
//! This is the Unix half of a seam. When Windows arrives, a daemon owning the
//! pty replaces the source and everything below it — the scanner, the events,
//! the screen — stays exactly as it is.

use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use devpit_pty::Told;
use tauri::Emitter;

use crate::happening::said;
use crate::sessions::SessionState;
use devpit_rpc::SessionLayout;
use tauri::State;

/// One pane being listened to.
struct Tap {
    /// Cleared to stop the reader; it checks between reads.
    listening: Arc<AtomicBool>,
}

/// Every pane this app is listening to, across projects.
#[derive(Default)]
pub struct Taps {
    open: Mutex<HashMap<String, Tap>>,
}

impl Taps {
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts listening to a pane, if it is not already.
    ///
    /// Idempotent because the callers are: ensuring a session walks every leaf
    /// it has, and does so on every window that opens the project.
    pub fn watch(
        &self,
        app: &tauri::AppHandle,
        server: &devpit_tmux::Server,
        leaf_id: &str,
        target: &str,
    ) {
        let Ok(mut open) = self.open.lock() else {
            return;
        };
        if open.contains_key(leaf_id) {
            return;
        }
        let Some(path) = fifo_for(leaf_id) else {
            return;
        };
        if make_fifo(&path).is_err() {
            return;
        }
        // Quoted: a path with a space in it would otherwise become two
        // arguments to the shell tmux runs this through.
        if server
            .pipe_pane(target, &format!("cat > '{}'", path.display()))
            .is_err()
        {
            return;
        }

        let listening = Arc::new(AtomicBool::new(true));
        open.insert(
            leaf_id.to_owned(),
            Tap {
                listening: Arc::clone(&listening),
            },
        );
        spawn_reader(app.clone(), leaf_id.to_owned(), path, listening);
    }

    /// Stops listening. The pane keeps running; only the copy ends.
    pub fn forget(&self, server: &devpit_tmux::Server, leaf_id: &str, target: &str) {
        let Ok(mut open) = self.open.lock() else {
            return;
        };
        let Some(tap) = open.remove(leaf_id) else {
            let _ = server.unpipe(target);
            return;
        };
        tap.listening.store(false, Ordering::Relaxed);
        let _ = server.unpipe(target);

        // The reader is blocked inside `read` and only looks at the flag
        // between reads, so a quiet pane would leave a thread waiting on a
        // fifo forever. One byte wakes it; it sees the flag and leaves.
        if let Some(path) = fifo_for(leaf_id) {
            if let Ok(mut waking) = std::fs::OpenOptions::new().write(true).open(&path) {
                use std::io::Write;
                let _ = waking.write_all(b"\0");
            }
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Where a pane's copy arrives.
fn fifo_for(leaf_id: &str) -> Option<PathBuf> {
    // The leaf id is ours and is alphanumeric, so it needs no escaping — but
    // a path is built from it, so anything else is refused rather than
    // sanitised into a name that collides with another pane's.
    if leaf_id.is_empty()
        || !leaf_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return None;
    }
    let root = devpit_core::Store::root().ok()?.join("taps");
    std::fs::create_dir_all(&root).ok()?;
    Some(root.join(format!("{leaf_id}.fifo")))
}

#[cfg(unix)]
fn make_fifo(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::ffi::OsStrExt;
    if path.exists() {
        return Ok(());
    }
    let name = std::ffi::CString::new(path.as_os_str().as_bytes())
        .map_err(|_| std::io::Error::other("a path with a nul in it"))?;
    // 0o600: the copy of a terminal's output is as private as the terminal.
    let made = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
    if made == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
fn make_fifo(_path: &std::path::Path) -> std::io::Result<()> {
    // Windows has no fifo, and no tmux to write to one. The daemon is what
    // fills this seam there; refusing here is what keeps the seam honest.
    Err(std::io::Error::other("no fifo on this platform"))
}

/// Reads a pane's copy for as long as it is being listened to.
fn spawn_reader(app: tauri::AppHandle, leaf_id: String, path: PathBuf, listening: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // Opened read *and* write: a fifo opened read-only ends the moment the
        // writer does, and the writer here is one `cat` per arming. Holding a
        // writer end ourselves means the reader outlives them and never sees a
        // false end of stream.
        let Ok(mut fifo) = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
        else {
            return;
        };
        let mut buffer = [0u8; 8192];
        while listening.load(Ordering::Relaxed) {
            let read = match fifo.read(&mut buffer) {
                Ok(0) => continue,
                Ok(read) => read,
                Err(_) => break,
            };
            // Cut into blocks there, and what it says about itself reported
            // from there — one scan of the stream, not two.
            crate::blocks::heard(&app, &leaf_id, &buffer[..read]);
        }
    });
}

/// What the window is told — the same event an attached pane sends, so the
/// screen has one thing to listen to whether or not it is looking.
pub(crate) fn report(app: &tauri::AppHandle, leaf_id: &str, told: Told) {
    if heard_by_tap(&told) {
        let _ = app.emit("terminal:happening", said(leaf_id, told));
    }
}

/// Which of the two streams speaks for what. The tap hears the pane itself —
/// prompts, commands, exits, titles, the cwd. A clipboard request is the one
/// thing tmux answers for its client (`set-clipboard`), so that one comes from
/// the attached side; heard on both, it would be copied twice.
pub(crate) fn heard_by_tap(told: &Told) -> bool {
    !matches!(told, Told::Clipboard(_))
}

/// Starts listening to every leaf the layout has.
///
/// Here rather than in `load_or_create` because that is also the read path:
/// arming a pipe is a side effect, and a function that only looks should not
/// have one. Idempotent, so calling it on every ensure is free.
pub(crate) fn listen(state: &State<SessionState>, project_id: &str, layout: &SessionLayout) {
    let Ok(server) = crate::sessions::tmux_server() else {
        return;
    };
    let session = devpit_tmux::Server::session_name(project_id);
    for (leaf_id, _) in layout.tree.leaves() {
        let target = devpit_tmux::Server::target(&session, leaf_id);
        state.taps.watch(state.app(), &server, leaf_id, &target);
    }
}

/// Ends whatever a pane is running, before its window goes.
///
/// Closing the tmux window is a `SIGHUP`, and a TUI agent installs a handler
/// for exactly that so it survives a disconnected terminal. Measured: a
/// process that ignores `SIGHUP` outlived `kill-window` and was reparented to
/// init, still working, with nothing left able to reach it. The close prompt
/// says "this will stop the agent's current work", and it has to be true.
///
/// A pane sitting at its prompt is left alone. Killing an idle shell's group
/// is violence for nothing; the window going is enough.
pub(crate) fn stop_whatever_runs(server: &devpit_tmux::Server, session: &str, leaf_id: &str) {
    let Ok(panes) = server.running(session) else {
        return;
    };
    let Some(pane) = panes.into_iter().find(|one| one.leaf_id == leaf_id) else {
        return;
    };
    let fronts = devpit_pty::looking(std::slice::from_ref(&pane.tty));
    let Some(front) = devpit_pty::front_on(&fronts, &pane.tty) else {
        return;
    };
    let command = devpit_pty::agents::program_of(&front.argv).unwrap_or(pane.command);
    if devpit_pty::agents::idle_shell(&command) {
        return;
    }
    devpit_pty::stop_group(front.pgid, devpit_pty::GRACE);
}
