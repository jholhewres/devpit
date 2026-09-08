//! The guard that keeps the agent CLI behind one crate.

use std::path::Path;

use walkdir::WalkDir;

use crate::Finding;

/// The one crate allowed to name the binary.
const OWNER: &str = "crates/agentcli";

/// Nothing outside `crates/agentcli` may invoke the agent CLI.
///
/// That surface is not documented by its vendor and will move. Spread across
/// the tree, a rename becomes a hunt; behind one crate it becomes one failing
/// test with the new shape written next to the old one.
pub fn only_one_crate_drives_the_agent(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    for area in ["crates", "apps"] {
        for entry in WalkDir::new(root.join(area))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            if path.strip_prefix(root).is_ok_and(|p| p.starts_with(OWNER)) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };

            for (number, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or(line);
                // Spawning it is the problem. A crate that merely mentions the
                // word in a message is not reaching for the binary.
                let spawns = code.contains("Command::new(\"claude\")")
                    || code.contains("Command::new(\"claude")
                    || (code.contains("\"claude\"") && code.contains("spawn"));
                if spawns {
                    findings.push(Finding {
                        file: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
                        line: number + 1,
                        what: format!(
                            "only {OWNER} may drive the agent CLI — go through it: `{}`",
                            code.trim()
                        ),
                    });
                }
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(area: &str, contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join(area);
        std::fs::create_dir_all(&src).expect("create tree");
        std::fs::write(src.join("lib.rs"), contents).expect("write");
        dir
    }

    /// The test that gives the guard its value.
    #[test]
    fn the_guard_catches_a_direct_spawn() {
        let dir = tree("crates/other/src", "let c = Command::new(\"claude\");\n");
        let findings = only_one_crate_drives_the_agent(dir.path());
        assert_eq!(findings.len(), 1, "the guard missed the spawn");
        assert!(findings[0].what.contains("crates/agentcli"));
    }

    /// The owner is exactly where that code belongs.
    #[test]
    fn the_guard_leaves_the_owning_crate_alone() {
        let dir = tree("crates/agentcli/src", "let c = Command::new(\"claude\");\n");
        assert!(only_one_crate_drives_the_agent(dir.path()).is_empty());
    }

    /// A noisy guard gets switched off. Naming the CLI in a message is not
    /// reaching for it.
    #[test]
    fn the_guard_stays_quiet_about_a_mention() {
        let dir = tree(
            "crates/other/src",
            "let msg = \"install claude to use this\";\n",
        );
        assert!(only_one_crate_drives_the_agent(dir.path()).is_empty());
    }

    #[test]
    fn the_real_repository_passes() {
        let findings = only_one_crate_drives_the_agent(&crate::workspace_root());
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
    }
}
