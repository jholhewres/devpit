//! Asking about one process by its pid, the same question on every platform.

/// Whether a process is still there. A process of someone else's, which this
/// one may not signal, still counts.
#[cfg(unix)]
pub fn alive(pid: i32) -> bool {
    // Signal 0 checks without sending anything; EPERM is a process that exists.
    pid > 0
        && (unsafe { libc::kill(pid, 0) } == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM))
}

/// Whether a process is still there: Windows has no signal 0, and `tasklist`
/// filtered to the pid lists it or says there is none.
#[cfg(windows)]
pub fn alive(pid: i32) -> bool {
    pid > 0
        && crate::host_env::command("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .is_ok_and(|out| String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\"")))
}

/// Asks a process to end: SIGTERM, which it may catch to clean up. Answers
/// whether the request was delivered.
#[cfg(unix)]
pub fn terminate(pid: i32) -> bool {
    pid > 0 && unsafe { libc::kill(pid, libc::SIGTERM) } == 0
}

/// Ends a process and the tree under it: Windows has no polite signal a
/// console program can be sent from outside its console.
#[cfg(windows)]
pub fn terminate(pid: i32) -> bool {
    pid > 0
        && crate::host_env::command("taskkill")
            .args(["/T", "/F", "/PID", &pid.to_string()])
            .output()
            .is_ok_and(|out| out.status.success())
}

#[cfg(test)]
mod tests {
    use super::alive;

    #[test]
    fn this_process_is_alive_and_no_pid_is_not() {
        assert!(alive(std::process::id() as i32));
        assert!(!alive(0));
        assert!(!alive(-1));
    }
}

/// Where a program is found on Windows: this process's `PATH`, then the one
/// Windows keeps for the user now — a tool installed after devpit started is
/// on that one only — under each extension Windows runs.
#[cfg(windows)]
pub fn found_on_path(name: &str) -> Option<std::path::PathBuf> {
    let mut dirs: Vec<std::path::PathBuf> = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect())
        .unwrap_or_default();
    dirs.extend(user_path());
    let has_extension = std::path::Path::new(name).extension().is_some();
    dirs.iter().find_map(|dir| {
        if has_extension {
            let whole = dir.join(name);
            return whole.is_file().then_some(whole);
        }
        ["exe", "cmd", "bat"]
            .iter()
            .map(|ext| dir.join(format!("{name}.{ext}")))
            .find(|candidate| candidate.is_file())
    })
}

/// The `Path` Windows keeps for this user, asked once.
#[cfg(windows)]
fn user_path() -> Vec<std::path::PathBuf> {
    static ASKED: std::sync::OnceLock<Vec<std::path::PathBuf>> = std::sync::OnceLock::new();
    ASKED
        .get_or_init(|| {
            crate::host_env::command("powershell.exe")
                .args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    "[Environment]::GetEnvironmentVariable('Path','User')",
                ])
                .output()
                .map(|out| {
                    std::env::split_paths(String::from_utf8_lossy(&out.stdout).trim()).collect()
                })
                .unwrap_or_default()
        })
        .clone()
}
