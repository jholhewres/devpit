//! Where the daemon listens and who it lets in.
//!
//! A unix socket, owner-only, in `$XDG_RUNTIME_DIR` (per-user, 0700, gone at
//! logout) or `~/.devpit/` without one. The mode is not trusted alone: every
//! connection's peer uid is asked of the kernel (`SO_PEERCRED`) and anyone but
//! the daemon's own user is turned away before a byte is read, because a
//! terminal is a shell and a shell is the account.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use crate::frame::{self, Frame, Kind};

/// The protocol the daemon speaks. Raised only when the wire changes: a
/// desktop update that keeps it keeps the daemon, and its terminals, alive.
pub const PROTOCOL: u32 = 1;
const NAME: &str = "devpit-ptyd.sock";

/// The socket's path: `$XDG_RUNTIME_DIR/devpit-ptyd.sock`, else
/// `<devpit home>/devpit-ptyd.sock`.
pub fn path(runtime_dir: Option<&Path>, devpit_home: &Path) -> PathBuf {
    match runtime_dir.filter(|dir| dir.is_absolute() && dir.is_dir()) {
        Some(dir) => dir.join(NAME),
        None => devpit_home.join(NAME),
    }
}

/// Listens at `path`, owner-only from creation. A socket left by a daemon
/// that died is removed; a live one is not, and binding then fails.
pub fn listen(path: &Path) -> io::Result<UnixListener> {
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                "a devpit-ptyd already listens there",
            ));
        }
        std::fs::remove_file(path)?;
    }
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    // Bound inside a directory only the owner can enter, made 0600 there, and
    // only then moved into place: there is no moment the socket is reachable
    // by others. Not by umask, which is the whole process's and would race
    // every other thread that creates a file meanwhile.
    let private = tempfile::Builder::new()
        .prefix(".devpit-ptyd-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(dir)?;
    let staged = private.path().join("sock");
    let listener = UnixListener::bind(&staged)?;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&staged, path)?;
    Ok(listener)
}

/// The uid on the other end of `stream`, as the kernel knows it.
pub fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
    use std::os::unix::io::AsRawFd;
    #[cfg(target_os = "linux")]
    {
        let mut cred = libc::ucred {
            pid: 0,
            uid: 0,
            gid: 0,
        };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let done = unsafe {
            libc::getsockopt(
                stream.as_raw_fd(),
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                (&mut cred as *mut libc::ucred).cast(),
                &mut len,
            )
        };
        if done != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(cred.uid)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let (mut uid, mut gid) = (0, 0);
        if unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(uid)
    }
}

/// Whether a peer may speak to the daemon: the same user, nobody else.
pub fn admitted(peer: u32, own: u32) -> bool {
    peer == own
}

pub fn own_uid() -> u32 {
    unsafe { libc::getuid() }
}

/// The daemon's side of the handshake: the first frame must be `Hello` with
/// this protocol. Answers `Welcome` either way, carrying its own version, so
/// a client of another version knows to replace the daemon, not retry.
pub fn greet(stream: &mut UnixStream) -> io::Result<()> {
    let first = frame::read(stream)?;
    let theirs = first
        .as_ref()
        .filter(|frame| frame.kind() == Some(Kind::Hello))
        .and_then(|frame| frame.bytes.get(..4))
        .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
    frame::write(
        stream,
        &Frame::new(0, Kind::Welcome, PROTOCOL.to_le_bytes().to_vec()),
    )?;
    match theirs {
        Some(PROTOCOL) => Ok(()),
        Some(other) => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("protocol {other}, not {PROTOCOL}"),
        )),
        None => Err(io::Error::new(io::ErrorKind::InvalidData, "no hello first")),
    }
}

/// The client's side: says its version, returns the daemon's.
pub fn hello(stream: &mut UnixStream, protocol: u32) -> io::Result<u32> {
    frame::write(
        stream,
        &Frame::new(0, Kind::Hello, protocol.to_le_bytes().to_vec()),
    )?;
    let answer = frame::read(stream)?
        .filter(|frame| frame.kind() == Some(Kind::Welcome))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no welcome"))?;
    let bytes = answer
        .bytes
        .get(..4)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "a short welcome"))?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

#[cfg(test)]
#[path = "socket_tests.rs"]
mod tests;
