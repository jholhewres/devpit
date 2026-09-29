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
