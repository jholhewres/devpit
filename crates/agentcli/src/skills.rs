//! The skills installed on this machine, by name.
//!
//! Referenced, never copied — the same rule as agents. A step names a skill;
//! this is what says whether that name exists, so saving a column can refuse a
//! typo instead of failing when a card lands on it.

use std::path::{Path, PathBuf};

/// Where a skill lives: a directory with a `SKILL.md` in it.
fn is_skill(dir: &Path) -> bool {
    dir.join("SKILL.md").is_file()
}

/// Every skill directory this machine has, from the person's own set and from
/// the plugins they installed.
pub fn sources() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    // Where the CLI actually keeps its configuration, which is not always
    // `~/.claude` — see `cli_config`.
    let cli = crate::cli_config::config_dir().unwrap_or_else(|| home.join(".claude"));
    sources_in(&cli, &home)
}

/// The skill directories of one installation of the CLI, plus the person's own.
pub fn sources_in(cli: &Path, home: &Path) -> Vec<PathBuf> {
    let mut dirs = walk(&cli.join("plugins/cache"), 0);
    for own in [cli.join("skills"), home.join(".devpit/skills")] {
        if own.is_dir() {
            dirs.push(own);
        }
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

fn walk(root: &Path, depth: usize) -> Vec<PathBuf> {
    if depth > 4 {
        return Vec::new();
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().is_some_and(|name| name == "skills") {
            found.push(path);
        } else {
            found.extend(walk(&path, depth + 1));
        }
    }
    found
}

/// The names, sorted and without repeats.
pub fn skills() -> Vec<String> {
    let mut names: Vec<String> = sources()
        .iter()
        .flat_map(|dir| std::fs::read_dir(dir).into_iter().flatten().flatten())
        .map(|entry| entry.path())
        .filter(|path| is_skill(path))
        .filter_map(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The names in `wanted` that this machine does not have.
pub fn missing(wanted: &[String], installed: &[String]) -> Vec<String> {
    wanted
        .iter()
        .filter(|name| !installed.contains(name))
        .cloned()
        .collect()
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
