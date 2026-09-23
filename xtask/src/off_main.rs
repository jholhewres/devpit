//! A command that makes the window wait.
//!
//! Tauri runs a plain `fn` command on the main thread — the one GTK draws the
//! window on. A command there that reads a file, asks git, lists processes or
//! opens the store holds every repaint and every click until it returns, and
//! on a busy machine that is the window "freezing" for seconds. An `async fn`
//! runs on the runtime instead, and the work goes through
//! `off_main::blocking`.
//!
//! The exceptions answer from memory and are listed by name, with the reason,
//! in `sync-commands.txt`.

use std::path::{Path, PathBuf};

use crate::Finding;

/// The commands allowed to stay synchronous. One `name reason` per line.
pub(crate) const ALLOWED: &str = include_str!("../sync-commands.txt");

const SOURCES: &str = "apps/desktop/src";

/// Every command is `async`, or listed with a reason.
pub fn no_command_holds_the_window(root: &Path) -> Vec<Finding> {
    held_in(root, ALLOWED)
}

fn held_in(root: &Path, allowed: &str) -> Vec<Finding> {
    let allowed = names(allowed);
    let mut findings = Vec::new();
    for file in sources(&root.join(SOURCES)) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let relative = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
        for (name, line) in sync_commands(&text) {
            if allowed.contains(&name) {
                continue;
            }
            findings.push(Finding {
                file: relative.clone(),
                line,
                what: format!(
                    "`{name}` is a plain `fn` command, which runs on the thread the window \
                     draws on — make it `async` over `off_main::blocking`, or list it in \
                     xtask/sync-commands.txt with why it never waits"
                ),
            });
        }
    }
    findings
}

/// The plain `fn` commands in one file, with the line each is declared on.
///
/// A command is the first `fn` after `#[tauri::command]`, whatever sits in
/// between — more attributes, an attribute over several lines, comments of
/// either kind — and whatever its visibility. Anything that is not an
/// `async fn` is reported: the default is that a command blocks, and only
/// `async` says otherwise.
pub(crate) fn sync_commands(text: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut armed = false;
    let mut in_comment = false;
    for (index, raw) in text.lines().enumerate() {
        let mut line = raw.trim();
        if in_comment {
            match line.find("*/") {
                Some(end) => {
                    in_comment = false;
                    line = line[end + 2..].trim();
                }
                None => continue,
            }
        }
        if let Some(start) = line.find("/*") {
            if !line[start..].contains("*/") {
                in_comment = true;
            }
            line = line[..start].trim();
        }
        if let Some(at) = line.find("#[tauri::command") {
            armed = true;
            // The attribute and the `fn` on one line.
            line = line[at..]
                .split_once(']')
                .map_or("", |(_, rest)| rest)
                .trim();
        }
        if !armed {
            continue;
        }
        let words: Vec<&str> = line.split_whitespace().collect();
        let Some(at) = words.iter().position(|word| *word == "fn") else {
            continue;
        };
        armed = false;
        if words[..at].contains(&"async") {
            continue;
        }
        let name: String = words
            .get(at + 1)
            .map(|rest| {
                rest.chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect()
            })
            .unwrap_or_default();
        found.push((name, index + 1));
    }
    found
}

fn sources(dir: &Path) -> Vec<PathBuf> {
    let mut all = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return all;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            all.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "rs")
            && !path
                .file_stem()
                .is_some_and(|stem| stem.to_string_lossy().ends_with("_tests"))
        {
            all.push(path);
        }
    }
    all.sort();
    all
}

fn names(list: &str) -> Vec<String> {
    list.lines()
        .map(|line| line.split('#').next().unwrap_or(line).trim())
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
        .collect()
}

#[cfg(test)]
#[path = "off_main_tests.rs"]
mod tests;
