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
//!   ask for.

use std::path::Path;

use crate::Finding;

const IDENTIFIER: &str = "dev.devpit.app";
const BASE: &str = "apps/desktop/tauri.conf.json";
const OVERLAY: &str = "apps/desktop/tauri.release.conf.json";
const CAPABILITIES: &str = "apps/desktop/capabilities/default.json";
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

    let granted: Vec<String> = json(root, CAPABILITIES)
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
            CAPABILITIES,
            format!("the window is granted {permission}: installing is not the page's to ask for"),
        );
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

fn json(root: &Path, relative: &str) -> serde_json::Value {
    std::fs::read_to_string(root.join(relative))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null)
}

#[cfg(test)]
#[path = "packaging_tests.rs"]
mod tests;
