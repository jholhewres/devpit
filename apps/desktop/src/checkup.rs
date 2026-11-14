//! What `devpit status` and `devpit diagnostics` read of this machine and the
//! running app, and the checks made of it, each with its fix. Nothing here
//! changes anything.

use std::path::Path;

use devpit_rpc::{Doing, IslandSession};

/// The running app's sessions, as the terminal is told them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub(crate) struct Sessions {
    pub running: usize,
    pub waiting: usize,
}

/// Free and total bytes on the disk devpit's home is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Disk {
    pub free: u64,
    pub total: u64,
}

/// Everything the checks and the status read, gathered once.
#[derive(Debug, Clone, Default)]
pub(crate) struct Facts {
    /// `None` when devpit is not open, or does not answer.
    pub sessions: Option<Sessions>,
    pub agent_cli: Option<String>,
    pub tmux: Option<String>,
    pub disk: Option<Disk>,
    /// The one-minute load average.
    pub load: Option<f64>,
    pub cpus: usize,
}

/// One check: what was looked at, whether it holds, and what to do when not.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Check {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
    pub fix: Option<String>,
}

/// Under this much free space a build or a new checkout stops half way.
pub(crate) const LOW_DISK: u64 = 5_000_000_000;

/// The sessions still going, and those stopped on the person.
pub(crate) fn sessions_of(all: &[IslandSession]) -> Sessions {
    Sessions {
        running: all
            .iter()
            .filter(|one| matches!(one.state, Doing::Open | Doing::Working | Doing::Waiting))
            .count(),
        waiting: all.iter().filter(|one| one.state == Doing::Waiting).count(),
    }
}

/// The checks, in the order they are shown.
pub(crate) fn checks(facts: &Facts, root: &Path) -> Vec<Check> {
    let mut all = vec![
        Check {
            name: "devpit is open",
            ok: facts.sessions.is_some(),
            detail: if facts.sessions.is_some() {
                "it answers".to_owned()
            } else {
                "it does not answer".to_owned()
            },
            fix: Some("run `devpit open`".to_owned()),
        },
        found(
            "claude is installed",
            facts.agent_cli.as_deref(),
            "install the claude CLI, then run `claude` once to sign in",
        ),
    ];
    // Windows carries its own; elsewhere a session needs the system's.
    if cfg!(unix) {
        all.push(found(
            "tmux is installed",
            facts.tmux.as_deref(),
            "install tmux with the system's package manager",
        ));
    }
    all.push(match facts.disk {
        Some(disk) => Check {
            name: "the disk has room",
            ok: disk.free >= LOW_DISK,
            detail: format!("{} free of {}", size(disk.free), size(disk.total)),
            fix: Some(format!(
                "free space on the disk that holds {}",
                root.display()
            )),
        },
        None => unknown("the disk has room"),
    });
    all.push(match facts.load {
        Some(load) => Check {
            name: "the load is normal",
            ok: load <= facts.cpus.max(1) as f64,
            detail: format!("{load:.2} on {} CPUs", facts.cpus),
            fix: Some("stop the sessions nobody is using; `devpit status` counts them".to_owned()),
        },
        None => unknown("the load is normal"),
    });
    for check in &mut all {
        if check.ok {
            check.fix = None;
        }
    }
    all
}

fn found(name: &'static str, at: Option<&str>, fix: &str) -> Check {
    Check {
        name,
        ok: at.is_some(),
        detail: at.unwrap_or("not found").to_owned(),
        fix: Some(fix.to_owned()),
    }
}

/// A check this system cannot make is not a failure.
fn unknown(name: &'static str) -> Check {
    Check {
        name,
        ok: true,
        detail: "not known on this system".to_owned(),
        fix: None,
    }
}

/// Bytes as `df -H` shows them.
pub(crate) fn size(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

/// What the checks read: the running app asked once, and this machine.
pub(crate) fn gather(root: &Path) -> Facts {
    let sessions = devpit_agentapi::client::ask(root, "status", serde_json::json!({}), root)
        .ok()
        .and_then(|answer| serde_json::from_value(answer).ok());
    machine(root, sessions)
}

/// The checks' facts with the app's sessions already known, as the app knows them.
pub(crate) fn machine(root: &Path, sessions: Option<Sessions>) -> Facts {
    Facts {
        sessions,
        agent_cli: devpit_agentcli::profile::found(devpit_agentcli::PROGRAM),
        tmux: devpit_agentcli::profile::found("tmux"),
        disk: disk(root),
        load: load(),
        cpus: std::thread::available_parallelism().map_or(1, usize::from),
    }
}

/// Asked of `df`, whose POSIX output reads the same on Linux and macOS.
#[cfg(unix)]
fn disk(root: &Path) -> Option<Disk> {
    let at = root.ancestors().find(|dir| dir.exists())?;
    let out = std::process::Command::new("df")
        .arg("-Pk")
        .arg(at)
        .output()
        .ok()?;
    df_disk(&String::from_utf8_lossy(&out.stdout))
}

/// `df -Pk`'s second line: total and available, in kilobytes.
pub(crate) fn df_disk(said: &str) -> Option<Disk> {
    let fields: Vec<&str> = said.lines().nth(1)?.split_whitespace().collect();
    let kb = |at: usize| fields.get(at)?.parse::<u64>().ok();
    Some(Disk {
        free: kb(3)? * 1024,
        total: kb(1)? * 1024,
    })
}

#[cfg(not(unix))]
fn disk(_root: &Path) -> Option<Disk> {
    None
}

#[cfg(unix)]
fn load() -> Option<f64> {
    let mut loads = [0f64; 3];
    // SAFETY: room for the three it may write.
    (unsafe { libc::getloadavg(loads.as_mut_ptr(), 3) } >= 1).then_some(loads[0])
}

#[cfg(not(unix))]
fn load() -> Option<f64> {
    None
}

#[cfg(test)]
#[path = "checkup_tests.rs"]
mod tests;
