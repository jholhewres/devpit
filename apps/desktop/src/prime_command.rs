//! One preparation command in a fresh worktree, with a ceiling on its time.

use std::collections::BTreeMap;
use std::path::Path;
use std::time::{Duration, Instant};

/// How long a preparation command runs when the project does not say.
const TIMEOUT_SECS: u64 = 600;
/// The lines of a command's output kept for the card.
const TAIL_LINES: usize = 20;

/// Runs `line` in `at`, telling `on_line` what it printed. `None` when it
/// succeeded; its exit code when not, 124 when it ran out of time.
pub(crate) fn ran(
    line: &str,
    at: &Path,
    share: &BTreeMap<String, String>,
    timeout: Option<u64>,
    log: &Path,
    on_line: &mut impl FnMut(&str),
) -> std::io::Result<Option<i32>> {
    on_line(&format!("$ {line}"));
    // Into a file, not a pipe: `pnpm i` says more than a pipe holds, and a
    // full pipe nobody reads hangs the command until the timeout.
    let out = std::fs::File::create(log)?;
    let mut child = devpit_pty::host_env::command("sh")
        .arg("-c")
        .arg(line)
        .current_dir(at)
        .envs(share)
        .stdout(out.try_clone()?)
        .stderr(out)
        .spawn()?;
    let limit = Duration::from_secs(timeout.unwrap_or(TIMEOUT_SECS));
    let began = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break Some(status);
        }
        if began.elapsed() >= limit {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    on_line(&tail(&std::fs::read_to_string(log).unwrap_or_default()));
    let _ = std::fs::remove_file(log);
    Ok(match status {
        Some(status) if status.success() => None,
        Some(status) => Some(status.code().unwrap_or(-1)),
        None => {
            on_line(&format!("stopped after {} s", limit.as_secs()));
            // 124, as `timeout` says it: not the command's own failure.
            Some(124)
        }
    })
}

/// The last lines of a command's output.
pub(crate) fn tail(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    lines[lines.len().saturating_sub(TAIL_LINES)..].join("\n")
}

#[cfg(test)]
#[path = "prime_command_tests.rs"]
mod tests;
