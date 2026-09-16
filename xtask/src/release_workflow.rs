//! The guard over the one job that holds a key.
//!
//! `release.yml` is the only workflow that signs, publishes, and can hand
//! somebody else's code the signing secret. None of what it promises is
//! visible from a diff of the app, so it is checked as text: a tag trigger, a
//! single permission, every action pinned to a commit, the secret in one step,
//! a tag that must already exist, and the manifests published after the files
//! they name.

use std::path::Path;

use crate::Finding;

const WORKFLOW: &str = ".github/workflows/release.yml";
const SECRET: &str = "secrets.TAURI_SIGNING_PRIVATE_KEY";

pub fn the_release_workflow_keeps_its_promises(root: &Path) -> Vec<Finding> {
    let Ok(text) = std::fs::read_to_string(root.join(WORKFLOW)) else {
        return vec![Finding {
            file: WORKFLOW.into(),
            line: 1,
            what: "there is no release workflow".to_owned(),
        }];
    };
    refusals(&text)
        .into_iter()
        .map(|(line, what)| Finding {
            file: WORKFLOW.into(),
            line,
            what,
        })
        .collect()
}

/// Every promise the workflow breaks, with the line to look at.
///
/// Its own function so a test can hand it a workflow rather than write one to
/// disk.
fn refusals(text: &str) -> Vec<(usize, String)> {
    let mut said = Vec::new();
    let at = |needle: &str| {
        text.lines()
            .position(|line| line.contains(needle))
            .map(|n| n + 1)
    };

    if at("tags:").is_none() {
        said.push((1, "does not run on a tag".to_owned()));
    }
    if !text.contains("contents: write") {
        said.push((1, "does not ask for contents: write".to_owned()));
    }

    for (number, line) in text.lines().enumerate() {
        let Some(used) = line.split("uses:").nth(1) else {
            continue;
        };
        let reference = used.split('#').next().unwrap_or("").trim();
        let pinned = reference
            .split_once('@')
            .is_some_and(|(_, sha)| sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()));
        if !pinned {
            said.push((
                number + 1,
                format!("{reference} is not pinned to a commit; a tag can be moved"),
            ));
        }
    }

    // Counted by step, not by mention: the one step that builds names both
    // the key and its password, and that is one holder, not two.
    let holders = steps_naming(text, SECRET);
    if holders == 0 {
        said.push((1, "never names the signing key".to_owned()));
    } else if holders > 1 {
        said.push((
            at(SECRET).unwrap_or(1),
            format!(
                "hands the signing key to {holders} steps; one builds, the rest do not need it"
            ),
        ));
    }

    if !text.contains("--verify-tag") {
        said.push((
            at("gh release create").unwrap_or(1),
            "publishes without --verify-tag: a release could name a tag nobody pushed".to_owned(),
        ));
    }

    match (at("gh release create"), at("gh release upload")) {
        (Some(created), Some(uploaded)) if uploaded < created => said.push((
            uploaded,
            "publishes the manifests before the files they name".to_owned(),
        )),
        (Some(_), None) => said.push((1, "never publishes the manifests".to_owned())),
        _ => {}
    }

    said
}

/// How many steps mention something. A step begins at `- ` in the steps list.
fn steps_naming(text: &str, needle: &str) -> usize {
    let mut holders = 0;
    let mut counted_here = false;
    for line in text.lines() {
        if line.starts_with("      - ") {
            counted_here = false;
        }
        if line.contains(needle) && !counted_here {
            holders += 1;
            counted_here = true;
        }
    }
    holders
}

#[cfg(test)]
#[path = "release_workflow_tests.rs"]
mod tests;
