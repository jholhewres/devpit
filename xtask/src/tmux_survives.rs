//! The guard that keeps the terminals alive through an update.
//!
//! tmux sessions outlive the window on purpose: that is what makes it possible
//! to install an update while an agent is mid-task in a pane. The app has
//! never killed the server, and this is what keeps it that way — `kill_server`
//! exists in `crates/tmux` because something has to be able to say it, and
//! nothing in `apps/desktop` ever should.

use std::path::Path;

use walkdir::WalkDir;

use crate::Finding;

/// What nothing in the app may name.
const NEVER: [&str; 2] = ["kill_server", "kill-server"];

pub fn the_app_never_kills_the_tmux_server(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let app = root.join("apps/desktop/src");

    for entry in WalkDir::new(&app).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        for (number, line) in text.lines().enumerate() {
            // A comment naming the call is documentation, not the app making
            // it — this guard's own explanation says the words out loud.
            if line.trim_start().starts_with("//") {
                continue;
            }
            if NEVER.iter().any(|never| line.contains(never)) {
                findings.push(Finding {
                    file: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
                    line: number + 1,
                    what: "the app kills the tmux server; the terminals are meant to outlive it"
                        .to_owned(),
                });
            }
        }
    }
    findings
}

#[cfg(test)]
#[path = "tmux_survives_tests.rs"]
mod tests;
