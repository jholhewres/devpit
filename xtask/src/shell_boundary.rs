//! The guard that keeps `crates/` from knowing the desktop shell.

use std::path::Path;

use walkdir::WalkDir;

use crate::Finding;

/// The guard that keeps the shell choice reversible.
///
/// Nothing under `crates/` may import `tauri`. If it needs to, the design is
/// wrong.
///
/// The reason is measured, not theoretical: WebKitGTK on Linux is the most
/// cited pain point in this stack. If it rules Tauri out, swapping shells has
/// to cost days — and it only does if the logic is not inside it.
///
/// Discovering this late is expensive: business logic that grew inside a
/// shell has to be extracted before the shell can be replaced.
pub fn core_does_not_know_the_shell(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let crates = root.join("crates");

    for entry in WalkDir::new(&crates)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        let is_source = path.extension().is_some_and(|e| e == "rs");
        let is_manifest = path.file_name().is_some_and(|n| n == "Cargo.toml");
        if !is_source && !is_manifest {
            continue;
        }

        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };

        for (number, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or(line);

            let offends = if is_manifest {
                // In a manifest the declared dependency is the problem, with
                // or without features or a path behind it.
                code.trim_start().starts_with("tauri")
                    || code.contains("tauri = ")
                    || code.contains("tauri-")
            } else {
                // In code: `use tauri::`, `extern crate tauri`, and the
                // qualified path `tauri::`. `tauri_something` does not count:
                // a name that merely starts the same is not the crate.
                code.contains("use tauri::")
                    || code.contains("extern crate tauri")
                    || code.contains("tauri::")
            };

            if offends {
                findings.push(Finding {
                    file: path.strip_prefix(root).unwrap_or(path).to_path_buf(),
                    line: number + 1,
                    what: format!(
                        "crates/ must not know the shell — move to apps/desktop: `{}`",
                        code.trim()
                    ),
                });
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a fake tree with one file under `crates/` and returns its root.
    fn tree(name: &str, contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let crate_dir = dir.path().join("crates/fake/src");
        std::fs::create_dir_all(&crate_dir).expect("create tree");
        std::fs::write(crate_dir.join(name), contents).expect("write");
        dir
    }

    /// The test that gives the guard its value.
    ///
    /// A guard only ever seen passing would pass just the same if it were
    /// broken. This one watches it fail on the exact thing it exists to catch.
    #[test]
    fn the_guard_catches_a_shell_import_in_the_core() {
        let dir = tree("lib.rs", "use tauri::Manager;\n\npub fn f() {}\n");

        let findings = core_does_not_know_the_shell(dir.path());

        assert_eq!(findings.len(), 1, "the guard missed the import");
        assert_eq!(findings[0].line, 1, "wrong line reported");
        assert!(
            findings[0].file.to_string_lossy().contains("fake"),
            "offending file not named: {}",
            findings[0].file.display()
        );
    }

    #[test]
    fn the_guard_catches_a_declared_manifest_dependency() {
        let dir = tempfile::tempdir().expect("tempdir");
        let crate_dir = dir.path().join("crates/fake");
        std::fs::create_dir_all(&crate_dir).expect("create tree");
        std::fs::write(
            crate_dir.join("Cargo.toml"),
            "[dependencies]\ntauri = { version = \"2\" }\n",
        )
        .expect("write");

        assert_eq!(core_does_not_know_the_shell(dir.path()).len(), 1);
    }

    /// A noisy guard gets switched off, which is the only way a guard really
    /// dies. One false positive a quarter is enough.
    #[test]
    fn the_guard_stays_quiet_about_what_is_not_the_shell() {
        let innocent = tree(
            "lib.rs",
            concat!(
                "// tauri::Manager appears here only in a comment\n",
                "use tauri_ish::Thing;\n",
                "pub fn restauring() {}\n",
            ),
        );

        let findings = core_does_not_know_the_shell(innocent.path());
        assert!(
            findings.is_empty(),
            "false positive: {}",
            findings
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    #[test]
    fn the_real_repository_passes() {
        let findings = core_does_not_know_the_shell(&crate::workspace_root());
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
    }
}
