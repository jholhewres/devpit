//! Everything a terminal's shell started, not only what is in front of it.
//!
//! Ending the foreground group left the rest running: a job sent to the
//! background, a child an agent started with `setsid`, a server a tool left
//! behind. They share the shell's session, or descend from it, so that is
//! what is ended — read from `/proc`, where the kernel keeps both.

use std::time::{Duration, Instant};

use crate::Stopped;

/// How long an agent gets to write out what it was doing before it is made
/// to stop. Longer than [`crate::GRACE`]: a pane closes off the main thread,
/// so nobody watches this wait, and an agent needs more than a shell does.
pub const AGENT_GRACE: Duration = Duration::from_millis(1500);

/// One row of the process table: what the kernel says about a pid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Proc {
    pub pid: u32,
    pub ppid: u32,
    pub sid: u32,
}

/// Reads `/proc/<pid>/stat`. The name sits in parentheses and may hold
/// spaces and parentheses of its own, so the fields are read after the last.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub fn parse_stat(stat: &str) -> Option<Proc> {
    let pid = stat.split_whitespace().next()?.parse().ok()?;
    let (_, rest) = stat.rsplit_once(')')?;
    // state, ppid, pgrp, session
    let mut fields = rest.split_whitespace();
    let state = fields.next()?;
    if state == "Z" {
        // Already dead, waiting to be reaped: nothing a signal changes.
        return None;
    }
    let ppid = fields.next()?.parse().ok()?;
    let _pgrp = fields.next()?;
    let sid = fields.next()?.parse().ok()?;
    Some(Proc { pid, ppid, sid })
}

/// The processes that belong to these shells: in one of their sessions, or
/// descended from one. Never a shell itself — the window going ends that —
/// and never `me`, which is this app.
pub fn members_of(table: &[Proc], leaders: &[u32], me: u32) -> Vec<u32> {
    let mut members: Vec<u32> = Vec::new();
    let mut frontier: Vec<u32> = leaders.to_vec();
    let belongs = |one: &Proc, members: &[u32]| {
        leaders.contains(&one.sid) || members.contains(&one.ppid) || leaders.contains(&one.ppid)
    };
    // Grown until nothing new joins: a grandchild names its parent, not the
    // shell, so one pass is not enough for a `setsid` two levels down.
    while !frontier.is_empty() {
        frontier.clear();
        for one in table {
            let skip = one.pid <= 1 || one.pid == me || leaders.contains(&one.pid);
            if !skip && !members.contains(&one.pid) && belongs(one, &members) {
                members.push(one.pid);
                frontier.push(one.pid);
            }
        }
    }
    members
}

/// What belongs to these shells right now, or `None` where nothing here can
/// read the process table.
pub fn members(leaders: &[u32]) -> Option<Vec<u32>> {
    let table = table()?;
    Some(members_of(&table, leaders, std::process::id()))
}

/// The children of `daemon` that `carry` a pane's name: its shell. A daemon
/// started from the pane carries the name too, but left it.
pub fn shells_of(table: &[Proc], daemon: u32, carry: impl Fn(u32) -> bool) -> Vec<u32> {
    table
        .iter()
        .filter(|one| one.ppid == daemon && carry(one.pid))
        .map(|one| one.pid)
        .collect()
}

/// Every process started with `name=value` in its environment, or `None`
/// where nothing here can read the process table.
pub fn started_with(name: &str, value: &str) -> Option<Vec<u32>> {
    let entry = format!("{name}={value}");
    let table = table()?;
    Some(
        (table.iter())
            .map(|one| one.pid)
            .filter(|pid| environ_has(*pid, entry.as_bytes()))
            .collect(),
    )
}

/// The children of `daemon` started with `name=value`: how a ptyd pane's
/// shell is found, since the protocol does not say its pid.
pub fn started_under(daemon: u32, name: &str, value: &str) -> Option<Vec<u32>> {
    let entry = format!("{name}={value}");
    let table = table()?;
    Some(shells_of(&table, daemon, |pid| {
        environ_has(pid, entry.as_bytes())
    }))
}

#[cfg(target_os = "linux")]
fn environ_has(pid: u32, entry: &[u8]) -> bool {
    std::fs::read(format!("/proc/{pid}/environ"))
        .is_ok_and(|environ| environ.split(|byte| *byte == 0).any(|one| one == entry))
}

#[cfg(not(target_os = "linux"))]
fn environ_has(_pid: u32, _entry: &[u8]) -> bool {
    false
}

/// The value `name` had when `pid` started, or `None` where nothing here can
/// read another process's environment.
#[cfg(target_os = "linux")]
pub fn environ_of(pid: u32, name: &str) -> Option<String> {
    let environ = std::fs::read(format!("/proc/{pid}/environ")).ok()?;
    let prefix = format!("{name}=");
    environ
        .split(|byte| *byte == 0)
        .find_map(|one| one.strip_prefix(prefix.as_bytes()))
        .and_then(|value| String::from_utf8(value.to_vec()).ok())
}

#[cfg(not(target_os = "linux"))]
pub fn environ_of(_pid: u32, _name: &str) -> Option<String> {
    None
}

/// Asks every member to end, waits up to `grace`, then insists.
pub fn stop_members(members: &[u32], grace: Duration) -> Stopped {
    if members.is_empty() {
        return Stopped::Already;
    }
    for pid in members {
        signal(*pid, TERM);
    }
    let deadline = Instant::now() + grace;
    loop {
        let left: Vec<u32> = members.iter().copied().filter(|pid| living(*pid)).collect();
        if left.is_empty() {
            return Stopped::Politely;
        }
        if Instant::now() >= deadline {
            for pid in left {
                signal(pid, KILL);
            }
            return Stopped::Forced;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

const TERM: i32 = 15;
const KILL: i32 = 9;

#[cfg(target_os = "linux")]
fn table() -> Option<Vec<Proc>> {
    let entries = std::fs::read_dir("/proc").ok()?;
    Some(
        entries
            .flatten()
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .bytes()
                    .all(|b| b.is_ascii_digit())
            })
            .filter_map(|entry| std::fs::read_to_string(entry.path().join("stat")).ok())
            .filter_map(|stat| parse_stat(&stat))
            .collect(),
    )
}

#[cfg(not(target_os = "linux"))]
fn table() -> Option<Vec<Proc>> {
    // No `/proc`. The caller falls back to the foreground group.
    None
}

/// Alive and not a zombie: a zombie answers signal 0 as though it ran.
#[cfg(target_os = "linux")]
fn living(pid: u32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat"))
        .ok()
        .and_then(|stat| parse_stat(&stat))
        .is_some()
}

#[cfg(not(target_os = "linux"))]
fn living(_pid: u32) -> bool {
    false
}

#[cfg(unix)]
fn signal(pid: u32, signal: i32) {
    // SAFETY: `kill` reads no memory; a pid that has gone is an error, not
    // undefined behaviour. Positive, so it names one process, never a group.
    unsafe {
        libc::kill(pid as i32, signal);
    }
}

#[cfg(not(unix))]
fn signal(_pid: u32, _signal: i32) {}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
