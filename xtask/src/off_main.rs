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

/// The `pub fn` commands in one file, with the line each is declared on.
///
/// A command is the first `fn` after `#[tauri::command]`; attributes and doc
/// comments may sit between them.
pub(crate) fn sync_commands(text: &str) -> Vec<(String, usize)> {
    let mut found = Vec::new();
    let mut armed = false;
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.starts_with("#[tauri::command") {
            armed = true;
            continue;
        }
        if !armed {
            continue;
        }
        if line.starts_with("#[") || line.starts_with("//") || line.is_empty() {
            continue;
        }
        armed = false;
        let signature = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub "))
            .unwrap_or(line);
        if let Some(rest) = signature.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            found.push((name, index + 1));
        }
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
