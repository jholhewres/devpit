//! The guard that keeps the Linux window in step with the base one.

use std::path::Path;

use crate::Finding;

/// Platform config replaces whole arrays instead of merging entries, so the
/// Linux window has to repeat the base window in full. That duplication drifts
/// silently: changing the size in the base leaves Linux on the old one, and a
/// partial entry drops title and decorations back to framework defaults —
/// which is how the system titlebar came back once already.
///
/// So the two are required to be identical except for the keys listed here.
pub fn platform_window_matches_the_base(root: &Path) -> Vec<Finding> {
    const MAY_DIFFER: [&str; 1] = ["transparent"];

    let base_path = root.join("apps/desktop/tauri.conf.json");
    let linux_path = root.join("apps/desktop/tauri.linux.conf.json");

    let read = |path: &Path| -> Option<serde_json::Value> {
        serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
    };

    let (Some(base), Some(linux)) = (read(&base_path), read(&linux_path)) else {
        return Vec::new();
    };

    let window = |v: &serde_json::Value| -> Option<serde_json::Map<String, serde_json::Value>> {
        v.get("app")?.get("windows")?.get(0)?.as_object().cloned()
    };

    let (Some(base_window), Some(linux_window)) = (window(&base), window(&linux)) else {
        return vec![Finding {
            file: linux_path
                .strip_prefix(root)
                .unwrap_or(&linux_path)
                .to_path_buf(),
            line: 1,
            what: "expected app.windows[0] in both configs".to_owned(),
        }];
    };

    let mut findings = Vec::new();
    let mut keys: Vec<&String> = base_window.keys().chain(linux_window.keys()).collect();
    keys.sort();
    keys.dedup();

    for key in keys {
        if MAY_DIFFER.contains(&key.as_str()) {
            continue;
        }
        if base_window.get(key) != linux_window.get(key) {
            findings.push(Finding {
                file: linux_path
                    .strip_prefix(root)
                    .unwrap_or(&linux_path)
                    .to_path_buf(),
                line: 1,
                what: format!(
                    "`{key}` drifted from tauri.conf.json ({:?} vs {:?}) — the platform \
                     override must repeat the base window in full",
                    base_window.get(key),
                    linux_window.get(key)
                ),
            });
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"{"app":{"windows":[{"label":"main","title":"X","decorations":false,"transparent":true}]}}"#;

    /// Writes a config pair under `apps/desktop/` and returns the root.
    fn configs(base: &str, linux: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let desktop = dir.path().join("apps/desktop");
        std::fs::create_dir_all(&desktop).expect("create tree");
        std::fs::write(desktop.join("tauri.conf.json"), base).expect("write base");
        std::fs::write(desktop.join("tauri.linux.conf.json"), linux).expect("write linux");
        dir
    }

    /// The exact failure that let the system titlebar back in: a partial
    /// override silently drops every key it does not repeat.
    #[test]
    fn the_guard_catches_a_partial_platform_window() {
        let dir = configs(
            BASE,
            r#"{"app":{"windows":[{"label":"main","transparent":false}]}}"#,
        );

        let findings = platform_window_matches_the_base(dir.path());

        assert_eq!(
            findings.len(),
            2,
            "expected title and decorations to be flagged"
        );
        let flagged = findings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            flagged.contains("decorations"),
            "missed decorations: {flagged}"
        );
        assert!(flagged.contains("title"), "missed title: {flagged}");
    }

    #[test]
    fn transparency_is_allowed_to_differ_and_nothing_else_is() {
        let dir = configs(
            BASE,
            r#"{"app":{"windows":[{"label":"main","title":"X","decorations":false,"transparent":false}]}}"#,
        );
        assert!(platform_window_matches_the_base(dir.path()).is_empty());
    }

    /// A project with no platform override is not a violation.
    #[test]
    fn the_guard_is_silent_without_a_platform_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(platform_window_matches_the_base(dir.path()).is_empty());
    }

    #[test]
    fn the_real_repository_passes() {
        let findings = platform_window_matches_the_base(&crate::workspace_root());
        assert!(
            findings.is_empty(),
            "{:?}",
            findings.iter().map(ToString::to_string).collect::<Vec<_>>()
        );
    }
}
