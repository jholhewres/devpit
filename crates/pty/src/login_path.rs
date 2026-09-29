//! The `PATH` the person's own shell has.
//!
//! devpit opened from the desktop's menu inherits the session's `PATH`, not the
//! one a login shell builds: `~/.local/bin`, `~/.cargo/bin`, a tool's own
//! folder added in `.zshrc`. A terminal of devpit's is a login shell and finds
//! them; an agent, a chat turn or `mcp list` started by devpit did not, and an
//! MCP server installed there read as broken.

use std::ffi::{OsStr, OsString};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

/// Long enough for a slow shell config; a shell that takes longer is left.
const WAITS: Duration = Duration::from_secs(4);
const MARK: &str = "__devpit_path__";

/// The shell's `PATH` joined with this process's, asked once. `None` when this
/// process was started from a shell, whose `PATH` it already has.
pub fn login_path() -> Option<&'static OsString> {
    static FOUND: OnceLock<Option<OsString>> = OnceLock::new();
    FOUND
        .get_or_init(|| {
            if std::env::var_os("SHLVL").is_some() {
                return None;
            }
            let said = asked()?;
            let own = std::env::var_os("PATH").unwrap_or_default();
            Some(joined(&said, &own))
        })
        .as_ref()
}

/// Runs the person's shell as a login, interactive shell and reads its `PATH`
/// off a marked line, past whatever its config prints.
fn asked() -> Option<OsString> {
    let shell = std::env::var_os("SHELL").filter(|one| !one.is_empty())?;
    let mut child = Command::new(shell)
        .args(["-lic", &format!("printf '\\n{MARK}%s\\n' \"$PATH\"")])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let started = Instant::now();
    while child.try_wait().ok()?.is_none() {
        if started.elapsed() > WAITS {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().ok()?;
    marked(&String::from_utf8_lossy(&output.stdout)).map(OsString::from)
}

/// The value on the marked line.
pub(crate) fn marked(said: &str) -> Option<String> {
    said.lines()
        .find_map(|line| line.strip_prefix(MARK))
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
}

/// The shell's entries first, then this process's that it lacks.
pub(crate) fn joined(shell: &OsStr, own: &OsStr) -> OsString {
    let mut all: Vec<std::path::PathBuf> = std::env::split_paths(shell).collect();
    for one in std::env::split_paths(own) {
        if !all.contains(&one) {
            all.push(one);
        }
    }
    std::env::join_paths(all).unwrap_or_else(|_| own.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{joined, marked};
    use std::ffi::OsStr;

    #[test]
    fn the_path_is_read_past_what_the_shell_config_prints() {
        let said = "welcome back\nsome banner __devpit_path__ no\n__devpit_path__/home/me/.tool/bin:/usr/bin\n";
        assert_eq!(marked(said).as_deref(), Some("/home/me/.tool/bin:/usr/bin"));
        assert_eq!(marked("nothing marked"), None);
    }

    #[test]
    fn the_shell_entries_lead_and_none_is_lost_or_doubled() {
        let path = joined(
            OsStr::new("/home/me/.tool/bin:/usr/bin"),
            OsStr::new("/usr/bin:/snap/bin"),
        );
        assert_eq!(path, OsStr::new("/home/me/.tool/bin:/usr/bin:/snap/bin"));
    }
}
