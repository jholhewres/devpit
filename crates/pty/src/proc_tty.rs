//! Which processes hold which terminal, read from `/proc` instead of `ps`.
//!
//! The sidebar and the status strip ask this every few seconds; a `ps` spawn
//! each time was most of what an idle devpit forked. The kernel already keeps
//! the same columns in `/proc/<pid>/stat`.

use crate::Front;

/// The columns of `/proc/<pid>/stat` this module reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtyStat {
    pub pid: u32,
    pub state: char,
    pub pgrp: u32,
    pub tty_nr: u32,
    /// The group that owns the terminal, `-1` read as 0.
    pub tpgid: u32,
}

impl TtyStat {
    /// In the group that owns its terminal: `ps`'s `+`.
    pub fn in_front(&self) -> bool {
        self.tpgid != 0 && self.pgrp == self.tpgid
    }
}

/// Reads one `/proc/<pid>/stat`. The name is in parentheses and may hold
/// either, so the fields are read after the last `)`.
pub fn parse_stat(stat: &str) -> Option<TtyStat> {
    let pid = stat.split_whitespace().next()?.parse().ok()?;
    let (_, rest) = stat.rsplit_once(')')?;
    // state ppid pgrp session tty_nr tpgid
    let mut fields = rest.split_whitespace();
    let state = fields.next()?.chars().next()?;
    let _ppid = fields.next()?;
    let pgrp = fields.next()?.parse().ok()?;
    let _session = fields.next()?;
    let tty_nr = fields.next()?.parse::<i64>().ok()?;
    let tpgid = fields.next()?.parse::<i64>().ok()?;
    Some(TtyStat {
        pid,
        state,
        pgrp,
        tty_nr: u32::try_from(tty_nr).unwrap_or(0),
        tpgid: u32::try_from(tpgid).unwrap_or(0),
    })
}

/// `tty_nr` as `ps` names it (`pts/8`), or `None` for no terminal or one
/// that is not a pseudo-terminal.
pub fn tty_name(tty_nr: u32) -> Option<String> {
    let major = (tty_nr >> 8) & 0xfff;
    let minor = (tty_nr & 0xff) | ((tty_nr >> 12) & 0xfff00);
    // UNIX98 pty slaves: majors 136–143.
    (136..=143)
        .contains(&major)
        .then(|| format!("pts/{}", (major - 136) * 256 + minor))
}

/// The bare name of a terminal however it was written: `/dev/pts/8` → `pts/8`.
pub fn bare(tty: &str) -> &str {
    tty.strip_prefix("/dev/").unwrap_or(tty)
}

/// The foreground rows on `ttys`, as `ps` would have listed them.
pub fn fronts_in(
    table: &[TtyStat],
    ttys: &[String],
    argv_of: impl Fn(u32) -> Option<Vec<String>>,
) -> Vec<Front> {
    let wanted: std::collections::HashSet<&str> = ttys.iter().map(|tty| bare(tty)).collect();
    table
        .iter()
        .filter(|row| row.in_front() && row.state != 'Z')
        .filter_map(|row| {
            let tty = tty_name(row.tty_nr).filter(|tty| wanted.contains(tty.as_str()))?;
            let argv = argv_of(row.pid).filter(|argv| !argv.is_empty())?;
            Some(Front {
                tty,
                pid: row.pid,
                pgid: row.pgrp,
                argv,
                resting: row.state == 'S',
            })
        })
        .collect()
}

/// Every process on each of `ttys`, by bare tty name.
pub fn on_ttys_in(
    table: &[TtyStat],
    ttys: &[String],
) -> std::collections::HashMap<String, Vec<u32>> {
    let wanted: std::collections::HashSet<&str> = ttys.iter().map(|tty| bare(tty)).collect();
    let mut found: std::collections::HashMap<String, Vec<u32>> = std::collections::HashMap::new();
    for row in table.iter().filter(|row| row.state != 'Z') {
        if let Some(tty) = tty_name(row.tty_nr).filter(|tty| wanted.contains(tty.as_str())) {
            found.entry(tty).or_default().push(row.pid);
        }
    }
    found
}

/// Every process with a terminal, from `/proc`.
#[cfg(target_os = "linux")]
pub fn table() -> Vec<TtyStat> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
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
        .filter(|row| row.tty_nr != 0)
        .collect()
}

/// A process's argument vector, from `/proc/<pid>/cmdline`.
#[cfg(target_os = "linux")]
pub fn argv(pid: u32) -> Option<Vec<String>> {
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    Some(
        raw.split(|byte| *byte == 0)
            .filter(|word| !word.is_empty())
            .map(|word| String::from_utf8_lossy(word).into_owned())
            .collect(),
    )
}

#[cfg(test)]
#[path = "proc_tty_tests.rs"]
mod tests;
