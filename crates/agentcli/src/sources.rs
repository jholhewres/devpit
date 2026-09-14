//! Where the agents on this machine live.
//!
//! Every directory, not the best one: picking a single "best" needs a rule
//! for which plugin wins, and any such rule is wrong for someone.

use std::path::Path;

/// Every directory on this machine that holds agents, the person's own set
/// first.
///
/// Every directory, not the best one. Picking a single "best" needs a rule for
/// which plugin wins, and any such rule is wrong for someone — one install had
/// a plugin with a single agent sorting above the set of nineteen. Reading all
/// of them, first-wins, gets the union with no rule at all.
pub fn seed_sources() -> Vec<std::path::PathBuf> {
    let Some(home) = dirs_home() else {
        return Vec::new();
    };
    let cli = crate::cli_config::config_dir().unwrap_or_else(|| home.join(".claude"));
    let mut dirs = walk_agent_dirs(&cli.join("plugins/cache"));
    dirs.sort();

    // Yours first, so it shadows a plugin's rather than the other way round.
    let mut all = Vec::new();
    for own in [home.join(".devpit/agents"), cli.join("agents")] {
        if own.is_dir() {
            all.push(own);
        }
    }
    all.extend(dirs);
    all
}

fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

/// Every `agents/` directory under a plugin cache, at any depth.
fn walk_agent_dirs(root: &Path) -> Vec<std::path::PathBuf> {
    fn visit(dir: &Path, depth: usize, found: &mut Vec<std::path::PathBuf>) {
        if depth > 4 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if path.file_name().is_some_and(|n| n == "agents") {
                found.push(path);
            } else {
                visit(&path, depth + 1, found);
            }
        }
    }
    let mut found = Vec::new();
    visit(root, 0, &mut found);
    found
}
