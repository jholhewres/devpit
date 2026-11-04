//! devpit-ptyd: the daemon that holds devpit's terminals apart from the
//! desktop, so a restart, crash or update of the window leaves them running.
//!
//! The desktop speaks to it over a unix socket ([`socket`]) in frames
//! ([`frame`]). Each pane keeps its last 2 MiB ([`history`]) and the modes it
//! is in ([`modes`]), so a window attaching late is put back where it was.
//! Output waits on acknowledgement past 1 MiB ([`pane::MOST_UNACKED`]), which
//! keeps memory flat under a flood and lets Ctrl+C through.

// Unix sockets and ptys: on Windows devpit's terminals stay on psmux, and
// only the wire format and the history are built there.
#[cfg(unix)]
pub mod attach;
#[cfg(unix)]
pub mod client;
#[cfg(unix)]
pub mod daemon;
pub mod frame;
pub mod history;
#[cfg(unix)]
pub mod launch;
pub mod modes;
#[cfg(unix)]
pub mod pane;
#[cfg(unix)]
pub mod socket;

/// Listens at `socket`, or where [`socket::path`] names for this devpit
/// home, and serves until killed. The exit code when it cannot.
#[cfg(unix)]
pub fn run(socket: Option<std::ffi::OsString>) -> i32 {
    let path = match socket {
        Some(path) => std::path::PathBuf::from(path),
        None => match devpit_core::Store::root() {
            Ok(home) => {
                let runtime = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
                socket::path(runtime.as_deref(), &home)
            }
            Err(err) => {
                eprintln!("devpit-ptyd: no home for devpit: {err}");
                return 2;
            }
        },
    };
    match socket::listen(&path) {
        Ok(listener) => {
            daemon::Daemon::new().serve(listener);
            0
        }
        Err(err) => {
            eprintln!("devpit-ptyd: cannot listen at {}: {err}", path.display());
            1
        }
    }
}

/// Terminals there stay on psmux; the daemon is unix sockets and ptys.
#[cfg(not(unix))]
pub fn run(_socket: Option<std::ffi::OsString>) -> i32 {
    eprintln!("devpit-ptyd runs on Linux and macOS only");
    2
}
