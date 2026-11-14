//! `diagnostics.environment` — what devpit sees of this machine, for a bug
//! report: the shell, the `PATH` children get, where the agent CLI and tmux
//! were found, the locale. No variable values beyond those.
//!
//! `diagnostics.read` — the Diagnostics page: devpit's processes with their
//! CPU and memory, the checks `devpit diagnostics` prints, and the error log.

use devpit_rpc::RpcError;

/// The report, as text to paste.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub text: String,
}

#[tauri::command]
#[specta::specta]
pub async fn diagnostics_environment() -> Result<EnvironmentReport, RpcError> {
    crate::off_main::blocking(|| Ok(EnvironmentReport { text: report() })).await
}

fn report() -> String {
    let asked = devpit_pty::login_path::asked();
    let path = devpit_pty::login_path::search_path();
    let found =
        |name: &str| devpit_agentcli::profile::found(name).unwrap_or("not found".to_owned());
    let var = |name: &str| std::env::var(name).unwrap_or("unset".to_owned());
    lines(&[
        ("devpit", env!("CARGO_PKG_VERSION").to_owned()),
        (
            "system",
            format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        ("shell", asked.shell.clone().unwrap_or("unset".to_owned())),
        (
            "login PATH",
            format!("{} in {} ms", asked.outcome, asked.took_ms),
        ),
        ("PATH used", path.to_string_lossy().into_owned()),
        ("claude", found(devpit_agentcli::PROGRAM)),
        ("tmux", found("tmux")),
        ("LANG", var("LANG")),
        ("LC_ALL", var("LC_ALL")),
        ("LC_CTYPE", var("LC_CTYPE")),
    ])
}

/// `name: value`, one per line.
fn lines(pairs: &[(&str, String)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{name}: {value}\n"))
        .collect()
}

/// One of devpit's own processes, as the Diagnostics page lists it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DevpitProcess {
    pub pid: u32,
    pub name: String,
    /// Tenths of a percent of one core, over the page's two looks.
    pub cpu_tenths: u32,
    pub memory_bytes: f64,
}

/// A check and its fix, as `devpit diagnostics` prints it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticCheck {
    pub name: String,
    pub ok: bool,
    pub detail: String,
    pub fix: Option<String>,
}

/// `diagnostics.read` — the Diagnostics page.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub processes: Vec<DevpitProcess>,
    pub checks: Vec<DiagnosticCheck>,
    /// The error log's most recent entries, newest first.
    pub log: Vec<String>,
    pub log_path: String,
}

/// Long enough between the two looks that a tick of rounding is not a core.
const LOOK_APART: std::time::Duration = std::time::Duration::from_millis(400);
/// The log lines the page shows.
const LOG_LINES: usize = 20;

#[tauri::command]
#[specta::specta]
pub async fn diagnostics_read() -> Result<Diagnostics, RpcError> {
    crate::off_main::blocking(|| {
        let root = devpit_core::Store::root()?;
        let sessions = crate::checkup::sessions_of(&crate::island_feed::now());
        let facts = crate::checkup::machine(&root, Some(sessions));
        let log = devpit_core::reports::ErrorLog::at(&root);
        Ok(Diagnostics {
            processes: processes(),
            checks: crate::checkup::checks(&facts, &root)
                .into_iter()
                .map(|check| DiagnosticCheck {
                    name: check.name.to_owned(),
                    ok: check.ok,
                    detail: check.detail,
                    fix: check.fix,
                })
                .collect(),
            log: log_tail(&log.read(), LOG_LINES),
            log_path: log.path().display().to_string(),
        })
    })
    .await
}

/// devpit's processes: this one and everything under it, and every other
/// process of this user named devpit (the terminal daemon, the MCP servers)
/// with theirs, measured twice for their CPU.
fn processes() -> Vec<DevpitProcess> {
    let pids = devpit_pty::usage::tree(&devpit_roots());
    let before = devpit_pty::usage::costs(&pids);
    let began = std::time::Instant::now();
    std::thread::sleep(LOOK_APART);
    let after = devpit_pty::usage::costs(&pids);
    let elapsed = began.elapsed().as_secs_f64();
    let mut all: Vec<DevpitProcess> = after
        .iter()
        .map(|now| {
            let was = before.iter().find(|was| was.pid == now.pid);
            process_of(name_of(now.pid), was, now, elapsed)
        })
        .collect();
    all.sort_by(|one, other| other.memory_bytes.total_cmp(&one.memory_bytes));
    all
}

/// One process from its two looks, `elapsed` seconds apart.
pub(crate) fn process_of(
    name: String,
    was: Option<&devpit_pty::usage::Cost>,
    now: &devpit_pty::usage::Cost,
    elapsed: f64,
) -> DevpitProcess {
    DevpitProcess {
        pid: now.pid,
        name,
        cpu_tenths: was.map_or(0, |was| {
            crate::watching::tenths(was.ticks, now.ticks, elapsed)
        }),
        // The cost is counted in KiB.
        memory_bytes: now.memory as f64 * 1024.0,
    }
}

/// Whether a process name is one of devpit's own.
pub(crate) fn is_devpit(name: &str) -> bool {
    name.starts_with("devpit")
}

#[cfg(target_os = "linux")]
fn devpit_roots() -> Vec<u32> {
    use std::os::unix::fs::MetadataExt as _;
    let me = std::fs::metadata("/proc/self").map(|meta| meta.uid()).ok();
    let mut roots = vec![std::process::id()];
    for entry in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        let mine = entry.metadata().ok().map(|meta| meta.uid()) == me;
        if mine && is_devpit(&name_of(pid)) {
            roots.push(pid);
        }
    }
    roots
}

#[cfg(not(target_os = "linux"))]
fn devpit_roots() -> Vec<u32> {
    vec![std::process::id()]
}

fn name_of(pid: u32) -> String {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .map(|name| name.trim().to_owned())
        .unwrap_or_else(|_| pid.to_string())
}

/// The error log's `most` most recently seen entries, newest first, one
/// line each: what kind, where, and what it said.
pub(crate) fn log_tail(entries: &[devpit_core::reports::Entry], most: usize) -> Vec<String> {
    let mut newest: Vec<&devpit_core::reports::Entry> = entries.iter().collect();
    newest.sort_by_key(|entry| std::cmp::Reverse(entry.last_seen));
    newest
        .into_iter()
        .take(most)
        .map(|entry| match &entry.location {
            Some(at) => format!("{} · {at}: {}", entry.kind, entry.message),
            None => format!("{}: {}", entry.kind, entry.message),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_log_shows_its_newest_entries_one_line_each() {
        let entry =
            |location: Option<&str>, message: &str, seen: u64| devpit_core::reports::Entry {
                fingerprint: String::new(),
                kind: "background".to_owned(),
                location: location.map(str::to_owned),
                message: message.to_owned(),
                stack: None,
                count: 1,
                first_seen: seen,
                last_seen: seen,
            };
        // As ErrorLog keeps them: the most recently seen first.
        let log = [
            entry(Some("stall_watch"), "the page has not run for 14s", 300),
            entry(None, "no place", 200),
            entry(Some("one"), "first", 100),
        ];
        assert_eq!(
            log_tail(&log, 2),
            vec![
                "background · stall_watch: the page has not run for 14s".to_owned(),
                "background: no place".to_owned(),
            ]
        );
        let mut shuffled = log.clone();
        shuffled.reverse();
        assert_eq!(log_tail(&shuffled, 2), log_tail(&log, 2));
        assert!(log_tail(&[], 20).is_empty());
    }

    #[test]
    fn a_process_is_measured_in_bytes_and_tenths_of_a_core() {
        let cost = |ticks| devpit_pty::usage::Cost {
            pid: 7,
            memory: 2048,
            shared: true,
            ticks,
        };
        let per_second = devpit_pty::usage::ticks_per_second();
        let shown = process_of(
            "devpit-ptyd".to_owned(),
            Some(&cost(0)),
            &cost(per_second / 2),
            1.0,
        );
        assert_eq!(
            shown.memory_bytes,
            2.0 * 1024.0 * 1024.0,
            "2048 KiB is 2 MiB"
        );
        assert_eq!(shown.cpu_tenths, 500, "half a core");
        assert_eq!(
            process_of("new".to_owned(), None, &cost(9), 1.0).cpu_tenths,
            0
        );
    }

    #[test]
    fn devpits_own_processes_are_the_ones_named_devpit() {
        assert!(is_devpit("devpit-desktop") && is_devpit("devpit-ptyd") && is_devpit("devpit"));
        assert!(!is_devpit("bash") && !is_devpit("claude"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_page_lists_this_process_with_its_memory() {
        let all = processes();
        let me = all
            .iter()
            .find(|one| one.pid == std::process::id())
            .expect("this process");
        assert!(me.memory_bytes > 0.0, "{me:?}");
    }

    #[test]
    fn the_report_names_what_a_bug_report_needs() {
        let text = report();
        for name in [
            "devpit:",
            "shell:",
            "login PATH:",
            "PATH used:",
            "claude:",
            "tmux:",
            "LANG:",
        ] {
            assert!(text.contains(name), "no {name} in\n{text}");
        }
    }
}
