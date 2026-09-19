//! The guard over what the bundle promises.
//!
//! Three things that are quiet until the day they are loud:
//!
//! - the **identifier** decides where the webview keeps its data, and the open
//!   tabs come back from that `localStorage`. Changing it empties every
//!   window on the next start;
//! - the **base bundle** stays a `.deb` with nothing signed, because every PR
//!   runs `make build` and the AppImage bundler reaches the network. The
//!   signing lives in the release overlay, and only there. Which bundles a
//!   release builds is the workflow's to say, because the answer differs by
//!   platform — an `appimage` on a Mac is an error nobody can act on — and a
//!   list here would be one list for three runners;
//! - the **window** is granted no `updater:` permission. The app checks and
//!   downloads from Rust; installing an update is not something a page may
//!   ask for;
//! - the **capability is scoped to a webview and not to a window**, because
//!   tauri enables a window-scoped capability on *every* webview inside that
//!   window whatever the `webviews` list says. The moment this window holds a
//!   second webview showing somebody else's page, `windows: ["main"]` hands
//!   devpit's whole command surface to it;
//! - the **image the Linux binary is built on** decides who can run it. A
//!   binary links against the glibc of the machine that made it and runs on
//!   that version or newer, never older, so the oldest image that can build
//!   this is the one that must. Raising it is a silent way to stop working on
//!   somebody else's machine, and it is invisible from a machine that is
//!   already newer than the floor.

use std::path::Path;

use crate::Finding;

const IDENTIFIER: &str = "dev.devpit.app";
const BASE: &str = "apps/desktop/tauri.conf.json";
const OVERLAY: &str = "apps/desktop/tauri.release.conf.json";
const CAPABILITIES: &str = "apps/desktop/capabilities";
const RELEASE: &str = ".github/workflows/release.yml";

pub fn the_bundle_says_what_it_ships(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut refuse = |file: &str, what: String| {
        findings.push(Finding {
            file: file.into(),
            line: 1,
            what,
        })
    };

    let base = json(root, BASE);
    if base.get("identifier").and_then(|id| id.as_str()) != Some(IDENTIFIER) {
        refuse(
            BASE,
            format!("identifier is not {IDENTIFIER}; the tabs come back from its localStorage"),
        );
    }
    let targets = base.pointer("/bundle/targets").and_then(|t| t.as_array());
    if targets.map(Vec::len) != Some(1) || targets.and_then(|t| t[0].as_str()) != Some("deb") {
        refuse(BASE, "bundle.targets is not exactly [deb]".to_owned());
    }
    if base.pointer("/bundle/createUpdaterArtifacts").is_some() {
        refuse(
            BASE,
            "createUpdaterArtifacts belongs to the release overlay".to_owned(),
        );
    }
    if base.pointer("/plugins/updater/pubkey").is_none() {
        refuse(BASE, "plugins.updater has no public key".to_owned());
    }

    let overlay = json(root, OVERLAY);
    if overlay.pointer("/bundle/targets").is_some() {
        refuse(
            OVERLAY,
            "the release overlay pins bundle.targets; the workflow names them per platform"
                .to_owned(),
        );
    }
    if overlay.pointer("/bundle/createUpdaterArtifacts") != Some(&serde_json::Value::Bool(true)) {
        refuse(
            OVERLAY,
            "the release overlay does not ask for updater artifacts".to_owned(),
        );
    }

    /* The bundles are named where the platform is known. A release that stops
    naming them builds whatever the base config says, which is a `.deb` and
    no updater artifact at all — a release nobody can update from. */
    let workflow = std::fs::read_to_string(root.join(RELEASE)).unwrap_or_default();
    for wanted in ["appimage,deb", "app,dmg"] {
        if !workflow.contains(&format!("bundles: {wanted}")) {
            refuse(RELEASE, format!("no runner is told to build {wanted}"));
        }
    }

    for image in above_the_floor(&workflow) {
        refuse(
            RELEASE,
            format!(
                "the Linux leg builds on {image}; {FLOOR} is the floor, and a \
                 newer image makes a binary that refuses to start on it"
            ),
        );
    }

    /* The unstable API, and whether anybody has read this version of it. */
    let lock = std::fs::read_to_string(root.join("Cargo.lock")).unwrap_or_default();
    let manifest =
        std::fs::read_to_string(root.join("apps/desktop/Cargo.toml")).unwrap_or_default();
    if manifest.contains("\"unstable\"") {
        match tauri_locked(&lock) {
            Some(found) if found == TAURI_READ => {}
            Some(found) => refuse(
                "apps/desktop/Cargo.toml",
                format!(
                    "tauri is {found} and the unstable API was last read at {TAURI_READ}. \
                     `unstable` means Window::add_child may change between minor releases — \
                     read what moved, then raise TAURI_READ in xtask/src/packaging.rs"
                ),
            ),
            None => refuse(
                "Cargo.lock",
                "no tauri version to check the unstable API against".to_owned(),
            ),
        }
    }

    /* Every file in the folder, not just `default.json`: tauri loads them all,
    so a second file scoped to a window would restore the hole while a guard
    that read one name reported ok. */
    for (named, capability) in capabilities_in(root) {
        if let Some(windows) = capability.get("windows").and_then(|w| w.as_array()) {
            let names: Vec<&str> = windows.iter().filter_map(|one| one.as_str()).collect();
            refuse(
                &named,
                format!(
                    "scoped to window {}; a window-scoped capability reaches every webview \
                     inside it, browser panes included. Scope it with `webviews`",
                    names.join(", ")
                ),
            );
        }
        if capability
            .get("webviews")
            .and_then(|w| w.as_array())
            .is_none_or(Vec::is_empty)
        {
            refuse(
                &named,
                "names no webviews; nothing in the window could call a command".to_owned(),
            );
        }

        let granted: Vec<String> = capability
            .get("permissions")
            .and_then(|p| p.as_array())
            .map(|p| {
                p.iter()
                    .filter_map(|one| one.as_str().map(ToOwned::to_owned))
                    .collect()
            })
            .unwrap_or_default();
        for permission in updater_permissions(&granted) {
            refuse(
                &named,
                format!("grants {permission}: installing is not the page's to ask for"),
            );
        }
    }

    findings
}

/// The updater permissions a capability file grants, if any.
///
/// Its own function so the test can hand it a list rather than a file.
fn updater_permissions(granted: &[String]) -> Vec<String> {
    granted
        .iter()
        .filter(|one| one.starts_with("updater:"))
        .cloned()
        .collect()
}

/// The tauri this repo has read the unstable API of.
///
/// `apps/desktop` turns on tauri's `unstable` feature for one thing:
/// `Window::add_child`, which the browser pane is made of. Unstable means the
/// API may change between *minor* releases, and `version = "2"` in a manifest
/// accepts every one of them — so a plain `cargo update` could change what
/// that call does with nobody reading anything.
///
/// This is the tripwire. Raising tauri means raising this line, and raising
/// this line means somebody looked.
const TAURI_READ: &str = "2.11.5";

/// The tauri the lockfile actually pins, if it can be read.
///
/// Its own function so the test can hand it a lockfile rather than the repo.
fn tauri_locked(lock: &str) -> Option<String> {
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line.trim() == "name = \"tauri\"" {
            return lines
                .next()?
                .trim()
                .strip_prefix("version = \"")?
                .strip_suffix('"')
                .map(ToOwned::to_owned);
        }
    }
    None
}

/// The oldest Ubuntu that can build this, and the one every Linux leg names.
///
/// Not older: Tauri v2 needs webkit2gtk **4.1**, and 20.04 carries only 4.0.
/// Not newer: 24.04 links `pidfd_spawnp` and the binary then requires
/// GLIBC_2.39, which 22.04 does not have.
const FLOOR: &str = "22.04";

/// The build images a workflow names that are newer than the floor.
///
/// Reads the matrix's `os:` entries and not `runs-on:`, because those are two
/// different questions: `runs-on` also names the job that only downloads
/// artifacts and writes manifests, which builds nothing and may sit on
/// whatever is current.
///
/// `ubuntu-latest` is refused by name. It is not newer today — it is newer
/// eventually, without a commit, which is the one failure a guard cannot
/// catch after the fact.
fn above_the_floor(workflow: &str) -> Vec<String> {
    workflow
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- os:").map(str::trim))
        .filter_map(|image| image.strip_prefix("ubuntu-"))
        .filter(|image| {
            /* `22.04-arm` is the same release on another architecture. */
            let release = image.split('-').next().unwrap_or(image);
            release != FLOOR
        })
        .map(|image| format!("ubuntu-{image}"))
        .collect()
}

/// Every capability file tauri would load, by path and parsed.
fn capabilities_in(root: &Path) -> Vec<(String, serde_json::Value)> {
    let folder = root.join(CAPABILITIES);
    let Ok(entries) = std::fs::read_dir(&folder) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|one| one == "json") {
            let named = format!("{CAPABILITIES}/{}", entry.file_name().to_string_lossy());
            let parsed = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| serde_json::from_str(&text).ok())
                .unwrap_or(serde_json::Value::Null);
            found.push((named, parsed));
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn json(root: &Path, relative: &str) -> serde_json::Value {
    std::fs::read_to_string(root.join(relative))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
#[path = "packaging_tests.rs"]
mod tests;
