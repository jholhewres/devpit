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
pub mod client;
#[cfg(unix)]
pub mod daemon;
pub mod frame;
pub mod history;
pub mod modes;
#[cfg(unix)]
pub mod pane;
#[cfg(unix)]
pub mod socket;
