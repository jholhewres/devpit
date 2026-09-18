//! The half of an update devpit never installs.
//!
//! A `.deb` is downloaded and verified here and installed by the person, with
//! a command they can read. Installing it means asking for root, and devpit
//! does not ask for root on anybody's behalf — the plugin would (pkexec, then
//! zenity, then sudo), which is exactly why `install()` is never called for
//! one.
//!
//! What this file is careful about is what it hands over: the bytes are
//! written where `_apt` can read them, checked again at the moment the command
//! is shown, and the command itself is built from binaries in the four
//! directories only root can write to.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Where a downloaded package waits. Outside `~/.devpit`, which is 0700: `_apt`
/// drops privileges to read the file and would be refused there.
pub(crate) fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
        .or_else(|| dirs_home().map(|home| home.join(".cache")))
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("devpit/updates")
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

/// Writes the package where the package manager can read it.
///
/// The folder is 0755 and the file 0644 — `_apt` drops privileges to read it,
/// and a file only this user can see would be refused with a warning nobody
/// can act on. Nothing secret is in a published package.
pub(crate) fn keep(bytes: &[u8], version: &str) -> std::io::Result<(PathBuf, String)> {
    let folder = cache_dir();
    std::fs::create_dir_all(&folder)?;
    let path = folder.join(format!("devpit_{version}_amd64.deb"));
    std::fs::write(&path, bytes)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(0o755))?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644))?;
    }

    Ok((path, digest_of(bytes)))
}

/// What a Debian package starts with. Every `.deb` is an `ar` archive whose
/// first member is `debian-binary`.
const DEBIAN_MAGIC: &[u8] = b"!<arch>\ndebian-binary";

/// Why a downloaded package is not one this app will name in a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotOurs {
    /// It is not there any more.
    Missing,
    /// Something other than a plain file: a link, a directory, a pipe.
    NotRegular,
    /// Outside the folder we wrote it to.
    Elsewhere,
    /// The bytes changed since they were verified.
    HashMismatch,
    /// Whatever it is, it is not a Debian package — and the plugin, not this
    /// app, chose the url it came from.
    NotADeb,
}

impl NotOurs {
    /// What a person is told, which is different in each case on purpose.
    pub(crate) fn said(self) -> &'static str {
        match self {
            NotOurs::Missing => "the downloaded package is no longer there",
            NotOurs::NotRegular => "the downloaded package is not a plain file",
            NotOurs::Elsewhere => "the downloaded package moved out of its folder",
            NotOurs::HashMismatch => "the downloaded package changed after it was verified",
            NotOurs::NotADeb => "what was downloaded is not a Debian package",
        }
    }
}

/// The sha-256 of some bytes, as hex.
pub(crate) fn digest_of(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Whether the file on disk is still the package that was verified.
///
/// Checked again at the moment the command is shown, not only when it was
/// written: a person may come back to this card hours later, and the file has
/// been sitting in a world-readable cache the whole time.
pub(crate) fn still_ours(path: &Path, folder: &Path, digest: &str) -> Result<(), NotOurs> {
    let found = std::fs::symlink_metadata(path).map_err(|_| NotOurs::Missing)?;
    if !found.is_file() {
        return Err(NotOurs::NotRegular);
    }
    if path.parent() != Some(folder) {
        return Err(NotOurs::Elsewhere);
    }
    let bytes = std::fs::read(path).map_err(|_| NotOurs::Missing)?;
    if digest_of(&bytes) != digest {
        return Err(NotOurs::HashMismatch);
    }
    if !bytes.starts_with(DEBIAN_MAGIC) {
        return Err(NotOurs::NotADeb);
    }
    Ok(())
}

/// The directories a command this app shows may name a binary from.
///
/// Not `$PATH`: the line ends up in a root shell, and a path somebody else can
/// prepend to is a command somebody else chose.
pub(crate) const TRUSTED: [&str; 4] = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];

/// The command a person runs to install the package, or why there is none.
///
/// No `-y`: the privileged step is theirs to read and agree to. The path is
/// single-quoted because a home directory with a space in it is not two
/// arguments, and refused outright unless absolute — a relative path means
/// something different in every directory they might paste this into.
pub(crate) fn install_command(package: &Path, dirs: &[&str]) -> Result<String, String> {
    if !package.is_absolute() {
        return Err("the package has no absolute path to name".to_owned());
    }
    let path = package.display().to_string();
    if path.contains('\'') {
        return Err("the package's path cannot be typed into a shell".to_owned());
    }

    let sudo = found_in(dirs, "sudo").ok_or_else(|| {
        "there is no sudo here, so install the package the way this machine does".to_owned()
    })?;
    let apt = found_in(dirs, "apt")
        .or_else(|| found_in(dirs, "dpkg"))
        .ok_or_else(|| "there is no apt or dpkg here to install it with".to_owned())?;

    let how = if apt.ends_with("dpkg") {
        "-i"
    } else {
        "install"
    };
    Ok(format!("{sudo} {apt} {how} '{path}'"))
}

/// The install, run by polkit rather than typed by the person.
///
/// The rule this was written under stands: devpit does not ask for root on
/// anybody's behalf. `pkexec` is not devpit asking — it hands the job to
/// polkit, which puts up the system's own dialog, takes the password itself,
/// and runs the one command it was given. Nothing here ever sees the password
/// and nothing here holds root.
///
/// `-y` here where the copied command has none, and for the same reason it
/// had none: there, the person reads the command before running it; here,
/// polkit has already asked them and there is no terminal to answer a prompt
/// in. A confirmation nobody can see is a hang.
///
/// `None` when this machine has no `pkexec`, and then the copied command is
/// the only honest answer.
pub(crate) fn elevated(package: &Path, dirs: &[&str]) -> Option<Vec<String>> {
    if !package.is_absolute() {
        return None;
    }
    let pkexec = found_in(dirs, "pkexec")?;
    let apt = found_in(dirs, "apt").or_else(|| found_in(dirs, "dpkg"))?;
    let how = if apt.ends_with("dpkg") {
        "-i"
    } else {
        "install"
    };
    /* Argv, not a shell line: the path goes in as one argument and nothing
    between here and the kernel gets to read it as anything else. */
    let mut argv = vec![pkexec, apt, how.to_owned()];
    if !argv[1].ends_with("dpkg") {
        argv.push("-y".to_owned());
    }
    argv.push(package.display().to_string());
    Some(argv)
}

fn found_in(dirs: &[&str], tool: &str) -> Option<String> {
    dirs.iter()
        .map(|dir| Path::new(dir).join(tool))
        .find(|path| path.is_file())
        .map(|path| path.display().to_string())
}

#[cfg(test)]
#[path = "update_deb_tests.rs"]
mod tests;
