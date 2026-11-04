//! Terminals held by devpit-ptyd, when the person turned it on.
//!
//! Off unless `terminal.backend` is `ptyd`. With it on, a new pane — a tab's
//! first, an agent's or a card's, or a split — is started in the daemon
//! instead of a tmux window, and outlives the desktop: a crash, a restart or
//! an update of the same protocol finds it running and attaches again. Every
//! pane that was made in tmux stays there until it is closed.
//!
//! Whatever goes wrong on the way — no daemon binary, a daemon that will not
//! start, one of another protocol — the pane is made in tmux as it always
//! was. A terminal on the old path is better than no terminal.
//!
//! A pane is told apart by its target: `ptyd:<leaf>` where tmux's would be
//! `session:window`. That keeps the layout's format, and every place that
//! reads the target for tmux finds nothing to do with this one.

use std::path::{Path, PathBuf};

use devpit_core::{preference, Store};

/// What a ptyd pane's target starts with, in place of a tmux session.
pub(crate) const TARGET_PREFIX: &str = "ptyd:";

/// The preference's value that turns it on.
pub(crate) const BACKEND: &str = "ptyd";

/// Whether new plain terminals go to the daemon.
pub(crate) fn wanted(store: &Store) -> bool {
    cfg!(unix)
        && store
            .preference(preference::TERMINAL_BACKEND)
            .ok()
            .flatten()
            .is_some_and(|value| value.trim() == BACKEND)
}

/// The target a ptyd pane is kept under in the layout.
pub(crate) fn target(leaf_id: &str) -> String {
    format!("{TARGET_PREFIX}{leaf_id}")
}

/// Whether a layout target is a ptyd pane's.
pub(crate) fn is_ptyd(target: &str) -> bool {
    target.starts_with(TARGET_PREFIX)
}

/// Whether `leaf_id` of `layout` is held by the daemon.
pub(crate) fn leaf_is_ptyd(layout: &devpit_rpc::SessionLayout, leaf_id: &str) -> bool {
    layout
        .tree
        .leaves()
        .into_iter()
        .any(|(id, target)| id == leaf_id && is_ptyd(target))
}

/// The leaves of `layout` the daemon holds.
pub(crate) fn ptyd_leaves(layout: &devpit_rpc::SessionLayout) -> Vec<String> {
    layout
        .tree
        .leaves()
        .into_iter()
        .filter(|(_, target)| is_ptyd(target))
        .map(|(id, _)| id.to_owned())
        .collect()
}

/// Ends the daemon's panes among `leaves`; the rest are tmux's to close.
pub(crate) fn close(leaves: &[String]) {
    #[cfg(unix)]
    if let Ok(home) = Store::root() {
        for leaf in leaves {
            kill(&home, pane_number(leaf));
        }
    }
    #[cfg(not(unix))]
    let _ = leaves;
}

/// The daemon's number for a leaf, stable across runs: the leaf id is what
/// the layout keeps, and the daemon only knows panes by number.
pub(crate) fn pane_number(leaf_id: &str) -> u32 {
    // FNV-1a, folded: not std's hasher, whose seed changes every run.
    let hash = leaf_id
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
        });
    // Never 0: the protocol's frames for no pane in particular carry it.
    ((hash >> 32) as u32 ^ hash as u32).max(1)
}

/// The binary to install the daemon from: devpit itself, which runs it as
/// `devpit ptyd <socket>`. Under an AppImage, the AppImage file, not the
/// mount it runs from, which goes away with the window.
fn binary() -> Option<PathBuf> {
    crate::agent_reach::exe()
}

#[cfg(unix)]
mod unix {
    use std::ffi::OsStr;
    use std::io;
    use std::os::unix::net::UnixStream;
    use std::path::Path;

    use devpit_ptyd::frame::{self, Frame, Kind};
    use devpit_ptyd::pane::Asked;

    /// Where this devpit home's daemon listens.
    pub(crate) fn socket_path(home: &Path) -> std::path::PathBuf {
        let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
        devpit_ptyd::socket::path(runtime.as_deref(), home)
    }

    /// A connection to this home's daemon, started when none answers.
    pub(crate) fn connect(home: &Path) -> io::Result<UnixStream> {
        let binary = super::binary();
        devpit_ptyd::launch::connect_or_start(
            binary.as_deref(),
            &[OsStr::new("ptyd")],
            home,
            &socket_path(home),
        )
    }

    /// One request, and the daemon's refusal if it answered with one. A
    /// request it took is answered by nothing at all, so a short wait that
    /// hears nothing is the yes.
    pub(crate) fn ask(stream: &mut UnixStream, one: Frame) -> io::Result<()> {
        let pane = one.pane;
        frame::write(stream, &one)?;
        stream.set_read_timeout(Some(std::time::Duration::from_millis(150)))?;
        let heard = frame::read(stream);
        stream.set_read_timeout(None)?;
        match heard {
            Ok(Some(answer)) if answer.pane == pane && answer.kind() == Some(Kind::Refused) => Err(
                io::Error::other(String::from_utf8_lossy(&answer.bytes).into_owned()),
            ),
            _ => Ok(()),
        }
    }

    /// Starts `asked` as pane `pane` in the daemon.
    pub(crate) fn spawn(home: &Path, pane: u32, asked: &Asked) -> io::Result<()> {
        let mut stream = connect(home)?;
        ask(&mut stream, Frame::new(pane, Kind::Spawn, asked.encode()))
    }

    /// Ends pane `pane`, if the daemon is there to be told.
    pub(crate) fn kill(home: &Path, pane: u32) {
        let at = socket_path(home);
        if let Ok(mut stream) = devpit_ptyd::socket::connect(&at) {
            let _ = frame::write(&mut stream, &Frame::new(pane, Kind::Kill, Vec::new()));
        }
    }

    /// Whether pane `pane` is running: attached to and asked nothing, a pane
    /// that is there answers with its replay, one that is not refuses.
    pub(crate) fn running(home: &Path, pane: u32) -> bool {
        let at = socket_path(home);
        let Ok(mut stream) = devpit_ptyd::socket::connect(&at) else {
            return false;
        };
        ask(&mut stream, Frame::new(pane, Kind::Attach, Vec::new())).is_ok()
    }
}

#[cfg(unix)]
pub(crate) use unix::{connect, kill, running, socket_path, spawn};

/// The shell a ptyd pane starts: the same wrapped shell tmux's windows run,
/// with the leaf's name and the project's shared environment handed to it.
#[cfg(unix)]
pub(crate) fn asked(
    leaf_id: &str,
    cwd: &Path,
    shared: Vec<(String, String)>,
) -> Result<devpit_ptyd::pane::Asked, devpit_rpc::RpcError> {
    let shell = crate::shell_launch::wrapped_shell()?;
    let mut env = vec![
        (devpit_tmux::PANE_ENV.to_owned(), leaf_id.to_owned()),
        ("COLORTERM".to_owned(), "truecolor".to_owned()),
    ];
    env.extend(shell.env);
    env.extend(shared);
    let mut argv = vec![shell.program];
    argv.extend(shell.args);
    Ok(devpit_ptyd::pane::Asked {
        rows: 24,
        cols: 80,
        cwd: cwd.to_string_lossy().into_owned(),
        argv,
        env,
    })
}

/// Makes leaf `leaf_id` a pane of the daemon, in `cwd`. `Err` when the
/// daemon cannot be had — the caller then makes it in tmux.
#[cfg(unix)]
pub(crate) fn open(
    home: &Path,
    leaf_id: &str,
    cwd: &Path,
    shared: Vec<(String, String)>,
) -> Result<String, String> {
    let asked = asked(leaf_id, cwd, shared).map_err(|err| err.message)?;
    spawn(home, pane_number(leaf_id), &asked).map_err(|err| err.to_string())?;
    Ok(target(leaf_id))
}

/// Brings a ptyd pane back where its shell last was, when the daemon lost it
/// — it died, or the machine restarted. The scrollback is gone; the folder
/// is not.
#[cfg(unix)]
pub(crate) fn revive(home: &Path, leaf_id: &str, cwd: &Path, shared: Vec<(String, String)>) {
    if running(home, pane_number(leaf_id)) {
        return;
    }
    if let Err(why) = open(home, leaf_id, cwd, shared) {
        eprintln!("devpit: {leaf_id} could not be started again in devpit-ptyd: {why}");
    }
}

/// A new pane made in the daemon of `home`, when the person turned it on
/// and the daemon can be had. Nothing, for tmux to make it as it always did.
fn in_ptyd_at(
    store: &Store,
    home: Option<&Path>,
    project_id: &str,
    leaf_id: &str,
    cwd: &Path,
) -> Option<String> {
    if !wanted(store) {
        return None;
    }
    #[cfg(unix)]
    {
        let home = home?;
        let shared = crate::checkout::shared_env_at(store, project_id, cwd);
        match open(home, leaf_id, cwd, shared) {
            Ok(target) => {
                // Heard from its first prompt, which is also what says it settled.
                crate::ptyd_tap::tap().heed(project_id, leaf_id);
                return Some(target);
            }
            Err(why) => {
                eprintln!("devpit: devpit-ptyd could not take a terminal, tmux does: {why}")
            }
        }
    }
    #[cfg(not(unix))]
    let _ = (home, project_id, leaf_id, cwd);
    None
}

/// The target of a new pane: the daemon's while the person keeps it on and
/// it answers, whatever the pane is for or split from; `in_tmux`'s otherwise.
pub(crate) fn new_pane(
    store: &Store,
    home: Option<&Path>,
    project_id: &str,
    leaf_id: &str,
    cwd: &Path,
    in_tmux: impl FnOnce() -> Result<String, devpit_rpc::RpcError>,
) -> Result<String, devpit_rpc::RpcError> {
    match in_ptyd_at(store, home, project_id, leaf_id, cwd) {
        Some(target) => Ok(target),
        None => in_tmux(),
    }
}

/// A ptyd leaf of a layout being loaded, started again where its shell last
/// was if the daemon lost it.
pub(crate) fn revive_at(
    store: &Store,
    project_id: &str,
    leaf_id: &str,
    was_in: Option<PathBuf>,
    cwd: &Path,
) {
    #[cfg(unix)]
    if let Ok(home) = Store::root() {
        let at = was_in.as_deref().unwrap_or(cwd);
        let shared = crate::checkout::shared_env_at(store, project_id, at);
        revive(&home, leaf_id, at, shared);
        crate::ptyd_tap::tap().heed(project_id, leaf_id);
    }
    #[cfg(not(unix))]
    let _ = (store, project_id, leaf_id, was_in, cwd);
}

#[cfg(test)]
#[path = "ptyd_link_tests.rs"]
pub(crate) mod tests;
