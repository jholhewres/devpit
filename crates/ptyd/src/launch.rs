//! Starting the daemon from the desktop, and finding it again.
//!
//! The daemon runs from a copy kept per protocol, `<home>/ptyd/v<PROTOCOL>/`,
//! not from the binary that started it: an AppImage's own files are a mount
//! that goes when the window does, and an update replaces the rest. A desktop
//! of the same protocol finds the copy and the daemon already running, and
//! leaves both alone — that is how an update keeps every terminal.
//!
//! It starts in a session of its own, with an environment it is handed name
//! by name: everything the desktop holds — tokens, keys, the AppImage's
//! library paths — would otherwise reach every shell the daemon ever starts,
//! for as long as it lives.

use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::socket;

/// The binary's name, beside the desktop's and in the copy.
pub const BINARY: &str = "devpit-ptyd";

/// What reaches the daemon of the desktop's environment, by name. A shell
/// needs to know who and where it is, how to speak, and how to reach the
/// session's display, bus and agent; nothing else of the desktop's is its
/// business. `LC_*` is let through too.
pub const ALLOWED: &[&str] = &[
    "HOME",
    "USER",
    "LOGNAME",
    "SHELL",
    "PATH",
    "LANG",
    "LANGUAGE",
    "TZ",
    "TMPDIR",
    "XDG_RUNTIME_DIR",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "XDG_STATE_HOME",
    "DISPLAY",
    "XAUTHORITY",
    "WAYLAND_DISPLAY",
    "DBUS_SESSION_BUS_ADDRESS",
    "SSH_AUTH_SOCK",
    "DEVPIT_HOME",
];

/// How long a daemon just started is given to answer.
const STARTING: Duration = Duration::from_secs(3);

/// The variables of `vars` the daemon may have.
pub fn allowed(vars: impl IntoIterator<Item = (OsString, OsString)>) -> Vec<(OsString, OsString)> {
    vars.into_iter()
        .filter(|(name, _)| {
            let name = name.to_string_lossy();
            ALLOWED.contains(&name.as_ref()) || name.starts_with("LC_")
        })
        .collect()
}

/// Where this protocol's copy of the daemon lives in a devpit home.
pub fn installed_at(home: &Path) -> PathBuf {
    home.join("ptyd")
        .join(format!("v{}", socket::PROTOCOL))
        .join(BINARY)
}

/// The copy of `from` for this protocol, made if there is none. One already
/// there is kept: it may be the one running, and a daemon of the same
/// protocol is the same daemon as far as any desktop can tell.
pub fn install(from: &Path, home: &Path) -> io::Result<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let at = installed_at(home);
    if at.is_file() {
        return Ok(at);
    }
    let dir = at.parent().unwrap_or(home);
    std::fs::create_dir_all(dir)?;
    // Copied beside it and renamed into place, so a daemon starting from a
    // half-written file is not a thing that can happen.
    let staged = tempfile::NamedTempFile::new_in(dir)?;
    std::fs::copy(from, staged.path())?;
    std::fs::set_permissions(staged.path(), std::fs::Permissions::from_mode(0o700))?;
    staged.persist(&at).map_err(|err| err.error)?;
    Ok(at)
}

/// Starts `exe` with `args`, detached: a session of its own, nothing on its
/// standard streams, and only the allowed part of `vars`. Waited for on a
/// thread, so it never lingers as a zombie of the desktop.
pub fn start(
    exe: &Path,
    args: &[&OsStr],
    vars: impl IntoIterator<Item = (OsString, OsString)>,
) -> io::Result<()> {
    let mut command = Command::new(exe);
    command
        .args(args)
        .env_clear()
        .envs(allowed(vars))
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: setsid is async-signal-safe and touches nothing of the parent;
    // it only takes the child out of the desktop's session and terminal.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn()?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

/// A connection to the daemon of `home`, started from `binary` when none
/// answers: the copy of it is run with `leading`, then the socket's path. A daemon of another protocol is not replaced — its terminals may
/// still be running — and comes back as `Unsupported`, for the caller to fall
/// back on what it used before.
pub fn connect_or_start(
    binary: Option<&Path>,
    leading: &[&OsStr],
    home: &Path,
    at: &Path,
) -> io::Result<UnixStream> {
    match socket::connect(at) {
        Ok(stream) => return Ok(stream),
        Err(err) if err.kind() == io::ErrorKind::Unsupported => return Err(err),
        Err(err) if err.kind() == io::ErrorKind::PermissionDenied => return Err(err),
        Err(_) => {}
    }
    let binary = binary
        .filter(|path| path.is_file())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "devpit-ptyd is not installed"))?;
    let exe = install(binary, home)?;
    let mut args = leading.to_vec();
    args.push(at.as_os_str());
    start(&exe, &args, std::env::vars_os())?;
    let deadline = Instant::now() + STARTING;
    loop {
        match socket::connect(at) {
            Ok(stream) => return Ok(stream),
            Err(err) if Instant::now() >= deadline => return Err(err),
            Err(_) => std::thread::sleep(Duration::from_millis(25)),
        }
    }
}

#[cfg(test)]
#[path = "launch_tests.rs"]
mod tests;
