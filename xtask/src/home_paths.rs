//! The guard that keeps a project's devpit folder spelled in one place.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::Finding;

/// The joins onto an agent CLI's own `projects/`, counted per file.
pub(crate) const CLI_ROOTS: &str = include_str!("../cli-roots.txt");

/// Where devpit's `projects/` may be spelled: the rule and its evidence.
const OWNERS: [&str; 2] = ["crates/core/src/home.rs", "crates/core/src/home_tests.rs"];

/// A path under devpit's `projects/` is built only by `ProjectHome`.
///
/// The folder is named in the database, not derived from the id, so a second
/// place spelling `projects/<id>` is a second place reading the wrong folder.
/// `~/.claude/projects` is spelled the same way and is the CLI's; those sites
/// are counted, exactly, in `xtask/cli-roots.txt`.
pub fn paths_come_from_home(root: &Path) -> Vec<Finding> {
    sites_in(CLI_ROOTS, root)
}

/// The guard over a given list, so a test can exercise it without the real one.
fn sites_in(list: &str, root: &Path) -> Vec<Finding> {
    let listed = parse(list);
    let mut found = BTreeMap::new();
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
            let relative = path.strip_prefix(root).unwrap_or(path);
            let name = relative.to_string_lossy().replace('\\', "/");
            if OWNERS.contains(&name.as_str()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };

            let sites: Vec<usize> = text
                .lines()
                .enumerate()
                .filter(|(_, line)| builds_projects_path(line))
                .map(|(number, _)| number + 1)
                .collect();
            let allowed = listed.get(&name).copied().unwrap_or(0);
            if sites.len() > allowed {
                for line in &sites {
                    findings.push(Finding {
                        file: relative.to_path_buf(),
                        line: *line,
                        what: format!(
                            "{} `projects/` path(s) here, {allowed} listed as an agent CLI's — \
                             build devpit's through `devpit_core::home::ProjectHome`",
                            sites.len()
                        ),
                    });
                }
            }
            if !sites.is_empty() {
                found.insert(name, sites.len());
            }
        }
    }

    // Exact, not a ceiling: a slot the CLI no longer uses is one a devpit
    // path could take without anything failing.
    for (name, allowed) in &listed {
        let has = found.get(name).copied().unwrap_or(0);
        if has < *allowed {
            findings.push(Finding {
                file: PathBuf::from(name),
                line: 1,
                what: format!("listed in xtask/cli-roots.txt for {allowed}, has {has} — lower it"),
            });
        }
    }

    findings
}

fn parse(list: &str) -> BTreeMap<String, usize> {
    list.lines()
        .filter_map(|line| {
            let entry = line.split('#').next().unwrap_or(line).trim();
            let (path, count) = entry.rsplit_once(char::is_whitespace)?;
            Some((path.trim().to_owned(), count.trim().parse().ok()?))
        })
        .collect()
}

/// Whether a line builds a path under a `projects/` folder.
///
/// `format!` counts as much as `join`: a grep for one misses the other.
fn builds_projects_path(line: &str) -> bool {
    let code = line.split("//").next().unwrap_or(line);
    code.contains("join(\"projects\")")
        || code.contains("\"projects/")
        || code.contains(".devpit/projects")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(file: &str, contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(file);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("create tree");
        std::fs::write(path, contents).expect("write");
        dir
    }

    /// The test that gives the guard its value.
    #[test]
    fn a_devpit_folder_joined_outside_home_is_caught() {
        let dir = tree(
            "apps/desktop/src/chat.rs",
            "let x = 1;\nlet dir = root.join(\"projects\").join(id);\n",
        );
        let findings = sites_in("", dir.path());
        assert_eq!(findings.len(), 1, "the guard missed the join");
        assert_eq!(findings[0].line, 2);
        assert!(findings[0].what.contains("ProjectHome"));
    }

    #[test]
    fn a_folder_spelled_with_format_is_caught_too() {
        let dir = tree(
            "apps/desktop/src/wsfiles.rs",
            "path: format!(\"projects/{id}\"),\n",
        );
        assert_eq!(sites_in("", dir.path()).len(), 1);
    }

    #[test]
    fn home_rs_is_where_the_folder_is_spelled() {
        let dir = tree("crates/core/src/home.rs", "root.join(\"projects\")\n");
        assert!(sites_in("", dir.path()).is_empty());
    }

    /// `~/.claude/projects` is the CLI's, and listing it is what lets it pass.
    #[test]
    fn a_listed_cli_install_root_passes() {
        let dir = tree(
            "crates/agentcli/src/transcript.rs",
            "let dir = installation.join(\"projects\").join(&folder);\n",
        );
        let list = "crates/agentcli/src/transcript.rs 1\n";
        assert!(sites_in(list, dir.path()).is_empty());
    }

    #[test]
    fn a_devpit_folder_beside_a_listed_cli_root_is_caught() {
        let dir = tree(
            "apps/desktop/src/session_search.rs",
            "installation.join(\"projects\")\nhome.join(\"projects\").join(id)\n",
        );
        let list = "apps/desktop/src/session_search.rs 1\n";
        assert_eq!(sites_in(list, dir.path()).len(), 2);
    }

    #[test]
    fn a_listing_with_nothing_behind_it_is_reported() {
        let dir = tree("crates/agentcli/src/transcript.rs", "let x = 1;\n");
        let list = "crates/agentcli/src/transcript.rs 1\n";
        assert_eq!(sites_in(list, dir.path()).len(), 1);
    }

    /// A noisy guard gets switched off. Naming the folder in a comment is not
    /// building it.
    #[test]
    fn a_comment_about_the_folder_is_not_a_path() {
        let dir = tree(
            "apps/desktop/src/projects.rs",
            "/// removes `~/.devpit/projects/`\nlet v = get(\"projects\");\n",
        );
        assert!(sites_in("", dir.path()).is_empty());
    }

    #[test]
    fn the_real_repository_passes() {
        let findings = paths_come_from_home(&crate::workspace_root());
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
    }
}
