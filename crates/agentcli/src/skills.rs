//! Where the skills of an installation live.
//!
//! Referenced, never copied — the same rule as agents. The panel that lists
//! them reads these directories; nothing here says which skills a turn may
//! use, because the CLI has no such setting.

use std::path::{Path, PathBuf};

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

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
