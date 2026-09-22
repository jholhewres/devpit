//! The environment a child of devpit should see, not the one devpit was given.
//!
//! An AppImage starts its binary with the environment pointed into its own
//! mount: `LD_LIBRARY_PATH`, `PYTHONHOME`, `GTK_PATH` and the rest name
//! `/tmp/.mount_…`. devpit needs them — WebKit loads its libraries through
//! them, so they cannot be cleared from the process itself — but a shell, an
//! agent or git inherits them too. Then `python3` cannot find its standard
//! library, and every program loads libraries built for another system. The
//! mount also moves on every start, so a tmux server started by one devpit
//! keeps paths into a mount the next one no longer has.
//!
//! What goes is decided by value, not by a list of names: an entry that points
//! into an AppImage mount is one the AppImage put there. A variable that had
//! entries of its own before keeps them.

use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

/// The variables that name the AppImage itself. A devpit started from one of
/// its terminals would read `APPIMAGE` and take itself for the installed
/// AppImage — and update that file instead of itself.
const ITSELF: [&str; 4] = ["APPIMAGE", "APPDIR", "ARGV0", "OWD"];

/// One change to a child's environment: a value to set, or `None` to remove.
pub type Change = (String, Option<String>);

/// What a child of this process needs changed, worked out once.
///
/// Empty unless this process runs from an AppImage, so a `.deb`, a macOS
/// bundle or a development build spawns exactly what it always did.
pub fn changes() -> &'static [Change] {
    static CHANGES: OnceLock<Vec<Change>> = OnceLock::new();
    CHANGES.get_or_init(|| match std::env::var("APPDIR") {
        Ok(appdir) if !appdir.is_empty() => cleaned(std::env::vars(), &mounts_under(&appdir)),
        _ => Vec::new(),
    })
}

/// `Command::new` for a program that runs on the person's behalf — a shell,
/// an agent, git, an opener — with the AppImage taken out of its environment.
pub fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut command = Command::new(program);
    scrub(&mut command);
    command
}

/// Applies [`changes`] to a command about to be spawned.
pub fn scrub(command: &mut Command) -> &mut Command {
    for (name, value) in changes() {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        };
    }
    command
}

/// The same, for a command that runs on a pty.
pub fn scrub_pty(command: &mut portable_pty::CommandBuilder) {
    for (name, value) in changes() {
        match value {
            Some(value) => command.env(name, value),
            None => command.env_remove(name),
        }
    }
}

/// Where AppImage mounts are made on this machine: beside this one when this
/// process is an AppImage, and in the temporary directory otherwise — a `.deb`
/// devpit can still inherit a tmux server an AppImage started.
pub fn mount_prefix() -> String {
    match std::env::var("APPDIR") {
        Ok(appdir) if !appdir.is_empty() => mounts_under(&appdir),
        _ => mounts_under(&std::env::temp_dir().join("devpit").to_string_lossy()),
    }
}

/// The prefix every AppImage mount beside this one starts with.
///
/// Not the mount itself: a tmux server outlives the devpit that started it,
/// and carries the path of a mount that has since moved.
pub fn mounts_under(appdir: &str) -> String {
    let parent = Path::new(appdir)
        .parent()
        .map(|parent| parent.to_string_lossy().into_owned())
        .unwrap_or_default();
    format!("{}/.mount_", parent.trim_end_matches('/'))
}

/// The changes that take every AppImage mount out of `vars`.
///
/// A path list keeps the entries that are not in a mount; a variable left
/// with none is removed, which is also what happens to a single path such as
/// `PYTHONHOME`.
pub fn cleaned(vars: impl IntoIterator<Item = (String, String)>, mounts: &str) -> Vec<Change> {
    let mut changes = Vec::new();
    for (name, value) in vars {
        if ITSELF.contains(&name.as_str()) {
            changes.push((name, None));
            continue;
        }
        if !value.contains(mounts) {
            continue;
        }
        let kept: Vec<&str> = value
            .split(':')
            .filter(|entry| !entry.is_empty() && !entry.starts_with(mounts))
            .collect();
        let value = (!kept.is_empty()).then(|| kept.join(":"));
        changes.push((name, value));
    }
    changes
}

#[cfg(test)]
#[path = "host_env_tests.rs"]
mod tests;
