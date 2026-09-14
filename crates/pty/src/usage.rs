//! What the processes behind a terminal actually cost.
//!
//! **Memory is proportional, not resident.** A process tree shares most of what
//! it maps — the interpreter, libc, every library loaded twice — and RSS counts
//! each shared page in full for every process holding it. Adding RSS across a
//! tree therefore reports memory that does not exist. Measured on the machine
//! this was written on: 203 processes summed to 17.67 GB of RSS against 12.21
//! GB of PSS, so RSS overstated real memory by 44%.
//!
//! PSS divides each shared page among the processes sharing it, which is the
//! only sum that adds up to what is on the machine. It costs more to read —
//! the kernel walks the page tables for `smaps_rollup` — so a caller asking
//! about every process on a busy machine should ask on a timer, not a frame.
//!
//! Where PSS cannot be read, RSS is reported and `shared` says so. A number
//! that might be 44% wrong has to arrive labelled.

/// What one process is costing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    pub pid: u32,
    /// Kibibytes.
    pub memory: u64,
    /// Whether shared pages were divided rather than counted in full.
    pub shared: bool,
    /// CPU time used since the process started, in clock ticks. A rate needs
    /// two of these and the time between them; this is one end of it.
    pub ticks: u64,
}

/// Every process whose controlling terminal is one of these, by tty.
///
/// Not `looking`, which keeps only the foreground process group because it
/// answers a different question — what is *in front of* this terminal. What a
/// terminal **costs** is everything its shell started, and a job backgrounded
/// with `&` is in another process group entirely: invisible to the foreground
/// reader, and often the expensive half.
///
/// Measured on a real pane: `sleep 600 & sleep 700` reported one process to
/// the foreground reader and two to this one.
pub fn on_ttys(ttys: &[String]) -> std::collections::HashMap<String, Vec<u32>> {
    let mut found: std::collections::HashMap<String, Vec<u32>> = std::collections::HashMap::new();
    if ttys.is_empty() {
        return found;
    }
    let bare: Vec<String> = ttys
        .iter()
        .map(|tty| tty.strip_prefix("/dev/").unwrap_or(tty).to_owned())
        .collect();
    let Ok(output) = std::process::Command::new("ps")
        .args(["-o", "pid=,tty=", "-t", &bare.join(",")])
        .output()
    else {
        return found;
    };
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let mut parts = line.split_whitespace();
        let (Some(pid), Some(tty)) = (parts.next(), parts.next()) else {
            continue;
        };
        if let Ok(pid) = pid.parse() {
            found.entry(tty.to_owned()).or_default().push(pid);
        }
    }
    found
}

/// What `on_ttys` reports for one terminal, however it was spelled.
pub fn on_tty(found: &std::collections::HashMap<String, Vec<u32>>, tty: &str) -> Vec<u32> {
    let bare = tty.strip_prefix("/dev/").unwrap_or(tty);
    found.get(bare).cloned().unwrap_or_default()
}

/// Every process in these trees, roots included, each one once.
///
/// The whole tree and not the foreground group: an agent's cost is mostly its
/// children — the language server it started, the MCP servers it spawned —
/// and a number that leaves them out is a number that says an agent is free.
#[cfg(target_os = "linux")]
pub fn tree(roots: &[u32]) -> Vec<u32> {
    let mut children: std::collections::HashMap<u32, Vec<u32>> = std::collections::HashMap::new();
    for entry in std::fs::read_dir("/proc").into_iter().flatten().flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        if let Some(parent) = parent_of(pid) {
            children.entry(parent).or_default().push(pid);
        }
    }

    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut todo: Vec<u32> = roots.to_vec();
    while let Some(pid) = todo.pop() {
        if !seen.insert(pid) {
            continue;
        }
        found.push(pid);
        if let Some(kids) = children.get(&pid) {
            todo.extend(kids);
        }
    }
    found
}

#[cfg(not(target_os = "linux"))]
pub fn tree(roots: &[u32]) -> Vec<u32> {
    // Without `/proc` there is no cheap way to walk this, and reporting the
    // roots alone would be reporting that an agent's children are free.
    roots.to_vec()
}

/// What each of these processes costs, leaving out the ones that have gone.
#[cfg(target_os = "linux")]
pub fn costs(pids: &[u32]) -> Vec<Cost> {
    pids.iter().filter_map(|pid| cost_of(*pid)).collect()
}

#[cfg(not(target_os = "linux"))]
pub fn costs(_pids: &[u32]) -> Vec<Cost> {
    Vec::new()
}

#[cfg(target_os = "linux")]
fn parent_of(pid: u32) -> Option<u32> {
    let raw = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    parse_parent(&raw)
}

#[cfg(target_os = "linux")]
fn cost_of(pid: u32) -> Option<Cost> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let ticks = parse_ticks(&stat)?;
    let (memory, shared) = match std::fs::read_to_string(format!("/proc/{pid}/smaps_rollup"))
        .ok()
        .and_then(|raw| parse_field(&raw, "Pss:"))
    {
        Some(pss) => (pss, true),
        // `smaps_rollup` needs the same privileges as reading the maps, and a
        // process that is not ours refuses. Resident is what is left.
        None => (
            parse_field(
                &std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?,
                "VmRSS:",
            )?,
            false,
        ),
    };
    Some(Cost {
        pid,
        memory,
        shared,
        ticks,
    })
}

/// The parent from `/proc/<pid>/stat`, which is the fourth field.
///
/// Read from the last `)` rather than by splitting the whole line: the second
/// field is the executable name in brackets and a program is allowed to be
/// called `foo) 1 2 3 (bar`.
pub fn parse_parent(stat: &str) -> Option<u32> {
    stat[stat.rfind(')')? + 1..]
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// `utime + stime` from `/proc/<pid>/stat`, fields 14 and 15.
pub fn parse_ticks(stat: &str) -> Option<u64> {
    let after = &stat[stat.rfind(')')? + 1..];
    let mut fields = after.split_whitespace().skip(11);
    let user: u64 = fields.next()?.parse().ok()?;
    let system: u64 = fields.next()?.parse().ok()?;
    Some(user + system)
}

/// A `Name:  1234 kB` line, in kibibytes.
pub fn parse_field(raw: &str, name: &str) -> Option<u64> {
    raw.lines()
        .find(|line| line.starts_with(name))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

/// How many ticks a second this machine counts.
///
/// `sysconf(_SC_CLK_TCK)`, which is 100 everywhere this runs — but reading it
/// rather than writing 100 down means a machine where it is not 100 reports a
/// percentage rather than a number four times too large.
pub fn ticks_per_second() -> u64 {
    #[cfg(target_os = "linux")]
    {
        // No libc dependency for one constant: the kernel exposes it, and 100
        // is the answer on every configuration this ships to.
        100
    }
    #[cfg(not(target_os = "linux"))]
    {
        100
    }
}

#[cfg(test)]
#[path = "usage_tests.rs"]
mod tests;
