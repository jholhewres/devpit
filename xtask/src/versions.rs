//! The guard that keeps one version in one place.
//!
//! The workspace's `[workspace.package] version` is the source: it is what
//! `app_info` reports (`env!("CARGO_PKG_VERSION")`), what the bundle is named
//! after, and what a release tag has to match. Every other file that used to
//! say it again is a file that can disagree — `web/package.json` sat at
//! `0.0.0` for as long as the rest said `0.1.0`.
//!
//! `tauri.conf.json` says nothing at all: with no `version` key the Tauri CLI
//! takes the Cargo one (tauri-utils config.rs, "If removed the version number
//! from `Cargo.toml` is used").

use std::path::Path;

use crate::Finding;

/// The manifests and package files that must agree, or say nothing.
const PACKAGE_FILES: [&str; 2] = ["package.json", "web/package.json"];

/// Crate manifests may not repeat the workspace version.
const MANIFESTS: [&str; 3] = [
    "apps/desktop/Cargo.toml",
    "crates/git/Cargo.toml",
    "crates/rpc/Cargo.toml",
];

pub fn the_version_has_one_source(root: &Path) -> Vec<Finding> {
    let Some(workspace) = workspace_version(&read(root, "Cargo.toml")) else {
        return vec![Finding {
            file: "Cargo.toml".into(),
            line: 1,
            what: "no [workspace.package] version to be the source".to_owned(),
        }];
    };

    let mut findings = Vec::new();
    for file in PACKAGE_FILES {
        let text = read(root, file);
        if let Some(what) = disagreeing(&workspace, json_version(&text).as_deref()) {
            findings.push(Finding {
                file: file.into(),
                line: line_of(&text, "\"version\""),
                what,
            });
        }
    }

    let tauri = read(root, "apps/desktop/tauri.conf.json");
    if json_version(&tauri).is_some() {
        findings.push(Finding {
            file: "apps/desktop/tauri.conf.json".into(),
            line: line_of(&tauri, "\"version\""),
            what: "names a version of its own; with none it takes the Cargo one".to_owned(),
        });
    }

    for file in MANIFESTS {
        let text = read(root, file);
        for line in repeated_lines(&text, &workspace) {
            findings.push(Finding {
                file: file.into(),
                line,
                what: format!("repeats the workspace version {workspace}"),
            });
        }
    }
    findings
}

fn read(root: &Path, relative: &str) -> String {
    std::fs::read_to_string(root.join(relative)).unwrap_or_default()
}

/// The version under `[workspace.package]`, which is the one everything else
/// is measured against.
fn workspace_version(manifest: &str) -> Option<String> {
    let after = manifest.split_once("[workspace.package]")?.1;
    let line = after.lines().find(|line| line.starts_with("version"))?;
    Some(line.split('"').nth(1)?.to_owned())
}

/// The `"version"` of a JSON file, without parsing the whole of it: these are
/// hand-written files and the key is at the top level.
fn json_version(text: &str) -> Option<String> {
    let after = text.split_once("\"version\"")?.1;
    Some(after.split('"').nth(1)?.to_owned())
}

/// What is wrong with what a file says, if anything.
fn disagreeing(workspace: &str, says: Option<&str>) -> Option<String> {
    match says {
        None => Some(format!("says no version; the workspace says {workspace}")),
        Some(said) if said != workspace => {
            Some(format!("says {said}; the workspace says {workspace}"))
        }
        Some(_) => None,
    }
}

/// Lines of a crate manifest that write the workspace version out again.
fn repeated_lines(manifest: &str, workspace: &str) -> Vec<usize> {
    let wanted = format!("version = \"{workspace}\"");
    manifest
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(&wanted))
        .map(|(at, _)| at + 1)
        .collect()
}

fn line_of(text: &str, key: &str) -> usize {
    text.lines()
        .position(|line| line.contains(key))
        .map(|at| at + 1)
        .unwrap_or(1)
}

#[cfg(test)]
#[path = "versions_tests.rs"]
mod tests;
