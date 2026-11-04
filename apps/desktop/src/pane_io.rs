//! Typing into a pane and asking where it stands, whichever holds it.
//!
//! A pane is tmux's or devpit-ptyd's, told apart by its target
//! ([`crate::ptyd_link::is_ptyd`]). What types into a pane comes through
//! here, so nothing hands tmux a target it has never heard of.

use std::path::PathBuf;

use devpit_rpc::{ErrorCode, RpcError, SessionLayout};
use devpit_tmux::Key;

/// Who holds a pane, read off its target.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Holder<'a> {
    Tmux {
        session: &'a str,
        window: &'a str,
    },
    /// devpit-ptyd, by its number for the pane.
    Ptyd(u32),
}

pub(crate) fn holder(target: &str) -> Holder<'_> {
    match target.strip_prefix(crate::ptyd_link::TARGET_PREFIX) {
        Some(leaf) => Holder::Ptyd(crate::ptyd_link::pane_number(leaf)),
        None => {
            let (session, window) = target.split_once(':').unwrap_or((target, ""));
            Holder::Tmux { session, window }
        }
    }
}

/// The target of `leaf_id`, in whichever of the project's tabs holds it.
pub(crate) fn target_of(project_id: &str, leaf_id: &str) -> Result<String, RpcError> {
    let layout = crate::sessions::holding(project_id, leaf_id)?;
    Ok(target_in(&layout, leaf_id))
}

/// A tmux leaf's target is rebuilt from its names rather than read back, so
/// a layout stored by an older build still reaches its window.
pub(crate) fn target_in(layout: &SessionLayout, leaf_id: &str) -> String {
    match crate::ptyd_link::leaf_is_ptyd(layout, leaf_id) {
        true => crate::ptyd_link::target(leaf_id),
        false => devpit_tmux::Server::target(
            &devpit_tmux::Server::session_name(&layout.project_id),
            leaf_id,
        ),
    }
}

/// Runs `line` at a pane's prompt: the line and the screen cleared first.
pub(crate) fn submit(target: &str, line: &str) -> Result<(), RpcError> {
    match holder(target) {
        Holder::Tmux { .. } => tmux()?.submit(target, line).map_err(internal),
        Holder::Ptyd(pane) => write(pane, submitted(line)),
    }
}

/// Hands `text` to whatever runs in a pane as one paste, and Enter after it
/// when `enter`.
pub(crate) fn paste(target: &str, text: &str, enter: bool) -> Result<(), RpcError> {
    match holder(target) {
        Holder::Tmux { .. } if enter => tmux()?.paste_and_send(target, text).map_err(internal),
        Holder::Tmux { .. } => tmux()?.paste(target, text).map_err(internal),
        Holder::Ptyd(pane) => write(pane, pasted(text, enter)),
    }
}

/// Presses `keys` on the program in a pane, in order.
pub(crate) fn press(target: &str, keys: &[Key]) -> Result<(), RpcError> {
    match holder(target) {
        Holder::Tmux { .. } => tmux()?.press(target, keys).map_err(internal),
        Holder::Ptyd(pane) => write(pane, pressed(keys)),
    }
}

/// Whether a pane's shell is what is in front, rather than a program it runs.
pub(crate) fn shell_in_front(target: &str) -> Result<bool, RpcError> {
    match holder(target) {
        Holder::Tmux { .. } => tmux()?.shell_in_front(target).map_err(internal),
        Holder::Ptyd(pane) => Ok(front(pane)
            .as_deref()
            .is_some_and(devpit_pty::agents::idle_shell)),
    }
}

/// An Enter where the shell is idle in front, so it draws its prompt again.
/// Answers whether it pressed one.
pub(crate) fn nudge(target: &str) -> Result<bool, RpcError> {
    match holder(target) {
        Holder::Tmux { .. } => tmux()?.nudge(target).map_err(internal),
        Holder::Ptyd(_) => {
            if !shell_in_front(target)? {
                return Ok(false);
            }
            press(target, &[Key::Enter])?;
            Ok(true)
        }
    }
}

/// The folder a pane's shell stands in: tmux asks the kernel, the daemon
/// says what the shell last told it.
pub(crate) fn cwd(target: &str) -> Option<PathBuf> {
    match holder(target) {
        Holder::Tmux { session, window } => tmux().ok()?.pane_path(session, window),
        Holder::Ptyd(pane) => told_cwd(pane).map(PathBuf::from),
    }
}

/// Scrolls a pane's history. Nothing for a ptyd pane: no tmux client draws
/// it, so xterm holds the history and its own wheel scrolls it.
pub(crate) fn scroll(target: &str, lines: i32) -> Result<(), RpcError> {
    match holder(target) {
        Holder::Tmux { .. } => tmux()?.scroll(target, lines).map_err(internal),
        Holder::Ptyd(_) => Ok(()),
    }
}

/// Has a pane's screen drawn again whole. Answers whether there was a window
/// on it to draw; the page asks again when there was not yet.
pub(crate) fn redraw(app: &tauri::AppHandle, target: &str) -> Result<bool, RpcError> {
    match holder(target) {
        Holder::Tmux { session, window } => tmux()?.redraw(session, window).map_err(internal),
        Holder::Ptyd(_) => {
            let leaf = target
                .strip_prefix(crate::ptyd_link::TARGET_PREFIX)
                .unwrap_or_default();
            jiggle(app, leaf)
        }
    }
}

/// A ptyd pane drawn again: one column narrower and back is a SIGWINCH,
/// which every full-screen program answers by drawing itself whole. The
/// size is the attached window's; without one there is nothing to draw on.
fn jiggle(app: &tauri::AppHandle, leaf: &str) -> Result<bool, RpcError> {
    let state = tauri::Manager::state::<crate::sessions::SessionState>(app);
    let Ok(live) = state.claims.live(leaf) else {
        return Ok(false);
    };
    let Some(size) = live.ptyd_size() else {
        return Ok(false);
    };
    if size.cols > 1 {
        live.resize(size.rows, size.cols - 1)?;
    }
    live.resize(size.rows, size.cols)?;
    Ok(true)
}

/// What is in front of a ptyd pane, as the daemon last said.
#[cfg(unix)]
fn front(pane: u32) -> Option<String> {
    crate::ptyd_watch::program(pane)
}

#[cfg(not(unix))]
fn front(_pane: u32) -> Option<String> {
    None
}

#[cfg(unix)]
fn told_cwd(pane: u32) -> Option<String> {
    crate::ptyd_watch::cwd(pane)
}

#[cfg(not(unix))]
fn told_cwd(_pane: u32) -> Option<String> {
    None
}

/// Kills what the shell's line holds and clears the screen: `C-e C-u C-l`,
/// as tmux's submit sends them.
const CLEAR: &[u8] = b"\x05\x15\x0c";
const PASTE_START: &[u8] = b"\x1b[200~";
const PASTE_END: &[u8] = b"\x1b[201~";

/// The bytes [`submit`] types into a ptyd pane.
pub(crate) fn submitted(line: &str) -> Vec<u8> {
    let mut bytes = CLEAR.to_vec();
    bytes.extend(pasted(line, true));
    bytes
}

/// The bytes [`paste`] types into a ptyd pane. Bracketed only when it is
/// several lines: with no screen to read whether the program asked for
/// bracketed paste, a single line goes as typed, which every program takes.
pub(crate) fn pasted(text: &str, enter: bool) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(text.len() + 16);
    match text.contains('\n') {
        true => {
            // A pasted end marker would end the paste early and run the rest.
            let text = text.replace("\x1b[201~", "");
            bytes.extend_from_slice(PASTE_START);
            // As tmux's paste-buffer does: a line ends in CR at a terminal.
            bytes.extend(text.replace("\r\n", "\n").replace('\n', "\r").bytes());
            bytes.extend_from_slice(PASTE_END);
        }
        false => bytes.extend_from_slice(text.as_bytes()),
    }
    if enter {
        bytes.push(b'\r');
    }
    bytes
}

/// The bytes a terminal sends for `keys`. Arrows in their normal-mode form:
/// with no screen there is no knowing a program asked for application mode,
/// and the programs that do take both.
pub(crate) fn pressed(keys: &[Key]) -> Vec<u8> {
    keys.iter()
        .flat_map(|key| match key {
            Key::Up => b"\x1b[A".as_slice(),
            Key::Down => b"\x1b[B",
            Key::Enter => b"\r",
            Key::Escape => b"\x1b",
        })
        .copied()
        .collect()
}

/// A frame's worth of input at most, so a long paste is several frames
/// rather than one the daemon refuses as too long.
#[cfg(unix)]
const INPUT_CHUNK: usize = 64 * 1024;

/// Bytes into a ptyd pane, over a connection of their own.
#[cfg(unix)]
fn write(pane: u32, bytes: Vec<u8>) -> Result<(), RpcError> {
    write_at(&devpit_core::Store::root()?, pane, &bytes)
}

/// [`write`], to the daemon of `home`. Connected only, never started: a
/// daemon that is not there holds no pane to type into. Nothing is waited
/// for — `Input` has no answer, and keys waited on are keys typed slowly.
#[cfg(unix)]
pub(crate) fn write_at(home: &std::path::Path, pane: u32, bytes: &[u8]) -> Result<(), RpcError> {
    use devpit_ptyd::frame::{self, Frame, Kind};
    let socket = crate::ptyd_link::socket_path(home);
    let mut stream = devpit_ptyd::socket::connect(&socket).map_err(|err| {
        RpcError::new(
            ErrorCode::Unsupported,
            format!("devpit-ptyd, which holds this terminal, did not answer: {err}"),
        )
    })?;
    for chunk in bytes.chunks(INPUT_CHUNK) {
        frame::write(&mut stream, &Frame::new(pane, Kind::Input, chunk.to_vec()))
            .map_err(|err| RpcError::internal(err.to_string()))?;
    }
    Ok(())
}

#[cfg(not(unix))]
fn write(_pane: u32, _bytes: Vec<u8>) -> Result<(), RpcError> {
    Err(RpcError::new(
        ErrorCode::Unsupported,
        "devpit-ptyd terminals are not on this platform",
    ))
}

fn tmux() -> Result<devpit_tmux::Server, RpcError> {
    crate::sessions::tmux_server()
}

fn internal(err: devpit_tmux::TmuxError) -> RpcError {
    RpcError::internal(err.to_string())
}

#[cfg(test)]
#[path = "pane_io_tests.rs"]
mod tests;
