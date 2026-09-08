//! The guard that keeps a dumping ground from being created.

use std::path::Path;

use walkdir::WalkDir;

use crate::Finding;

/// Names that carry no information.
///
/// `utils.rs` starts as one function nobody could place and ends as the file
/// nobody can reason about, because nothing in the name says what does *not*
/// belong in it. A name from the domain — `tab_group_state`, not
/// `tab_helpers` — answers that question by itself.
const NAMES_THAT_SAY_NOTHING: &[&str] = &["utils", "helpers", "common", "misc", "shared"];

/// Source files and directories may not be named after nothing.
pub fn nothing_is_named_after_nothing(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();

    for area in ["crates", "web/src", "apps"] {
        for entry in WalkDir::new(root.join(area))
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let is_source = entry.file_type().is_file()
                && path
                    .extension()
                    .is_some_and(|e| e == "rs" || e == "ts" || e == "tsx");
            if !entry.file_type().is_dir() && !is_source {
                continue;
            }
            if !NAMES_THAT_SAY_NOTHING.contains(&name) {
                continue;
            }
            findings.push(Finding {
                file: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
                line: 1,
                what: format!(
                    "`{name}` names no concept and becomes a dumping ground — \
                     name it after what it holds"
                ),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_with(name: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let src = dir.path().join("crates/fake/src");
        std::fs::create_dir_all(&src).expect("create tree");
        std::fs::write(src.join(name), "pub fn f() {}\n").expect("write");
        dir
    }

    /// The test that gives the guard its value: it watches it fail on the
    /// exact thing it exists to catch.
    #[test]
    fn the_guard_catches_a_file_named_after_nothing() {
        let dir = tree_with("utils.rs");
        let findings = nothing_is_named_after_nothing(dir.path());
        assert_eq!(findings.len(), 1, "the guard missed utils.rs");
        assert!(findings[0].what.contains("dumping ground"));
    }

    /// A directory is the worse case: it invites more of them.
    #[test]
    fn the_guard_catches_a_directory_too() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("crates/fake/src/helpers")).expect("create tree");
        assert_eq!(nothing_is_named_after_nothing(dir.path()).len(), 1);
    }

    /// A noisy guard gets switched off, which is the only way a guard really
    /// dies. This one stays quiet about a name that says something.
    #[test]
    fn the_guard_stays_quiet_about_a_real_name() {
        let dir = tree_with("tab_group_state.rs");
        assert!(nothing_is_named_after_nothing(dir.path()).is_empty());
    }

    #[test]
    fn the_real_repository_passes() {
        let findings = nothing_is_named_after_nothing(&crate::workspace_root());
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
    }
}
