//! The guard over a pane nobody can open.
//!
//! A pane is declared in three places and each of them is silent about the
//! others: `paneList.tsx` names it, `paneMounts.tsx` draws it, and
//! `Sidebar.tsx` is where a person clicks to get one. The first two have a
//! test between them. The third had nothing, and the browser pane shipped
//! through the whole of a plan — mounted, tested, in the generated contract —
//! while being reachable only from the command palette.
//!
//! What found it was the e2e, saying `no button says Browser`. This is the
//! cheaper version of that sentence.
//!
//! Some panes are opened by acting on something rather than by asking for one,
//! and those are listed by name with the reason. The list is short on purpose:
//! it is an exemption, and an exemption nobody reads is how the next pane goes
//! missing.

use std::path::Path;

use crate::Finding;

const PANES: &str = "web/src/shell/paneList.tsx";
const SHELL: &str = "web/src/shell";

/// The palette is deliberately not read.
///
/// `paletteReach.tsx` offers every pane in `PANES` generically, so counting it
/// would make this guard say yes to everything — and "reachable from the
/// command palette" is exactly the state the browser pane shipped in. What is
/// being asked here is whether there is something to *click*.
const PALETTE: &str = "paletteReach.tsx";

/// Panes that are never asked for, and what asks for them instead.
const OPENED_BY_SOMETHING_ELSE: [(&str, &str); 4] = [
    ("file", "a file opens from the tree, the palette or a diff"),
    ("diff", "a diff opens from the changes panel or a card"),
    ("note", "a note opens from the Notes capability's own list"),
    (
        "drawing",
        "a drawing opens from the Excalidraw capability's own list",
    ),
];

/// Every pane name `paneList.tsx` declares.
pub(crate) fn declared(list: &str) -> Vec<String> {
    list.lines()
        .filter_map(|line| {
            let at = line.find("{ name: '")?;
            let rest = &line[at + "{ name: '".len()..];
            let end = rest.find('\'')?;
            Some(rest[..end].to_owned())
        })
        .collect()
}

/// Every pane some screen can open, by either of the two names a click uses.
///
/// `open('x')` in the sidebar and `show('x')` everywhere else — the Manager
/// opens from the top bar and Capabilities from its own row, and both are
/// perfectly reachable. The question is whether *something* offers it, not
/// whether the sidebar does.
pub(crate) fn offered(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for opener in ["open('", "show('"] {
        let mut rest = source;
        while let Some(at) = rest.find(opener) {
            rest = &rest[at + opener.len()..];
            if let Some(end) = rest.find('\'') {
                found.push(rest[..end].to_owned());
            }
        }
    }
    found
}

/// Everything the shell's screens can open, the palette excluded.
fn offered_anywhere(root: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(root.join(SHELL)) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == PALETTE || name.contains(".test.") {
            continue;
        }
        if path
            .extension()
            .is_some_and(|one| one == "tsx" || one == "ts")
        {
            found.extend(offered(&std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    found
}

/// The panes a person has no way to click for.
pub(crate) fn unreachable(list: &str, offered: &[String]) -> Vec<String> {
    declared(list)
        .into_iter()
        .filter(|pane| {
            !offered.contains(pane)
                && !OPENED_BY_SOMETHING_ELSE
                    .iter()
                    .any(|(named, _)| named == pane)
        })
        .collect()
}

pub fn every_pane_can_be_opened(root: &Path) -> Vec<Finding> {
    let list = std::fs::read_to_string(root.join(PANES)).unwrap_or_default();
    let offered = offered_anywhere(root);

    let mut findings = Vec::new();
    if list.is_empty() || offered.is_empty() {
        findings.push(Finding {
            file: PANES.into(),
            line: 1,
            what: "the pane list or the shell's screens could not be read, so nothing was checked"
                .to_owned(),
        });
        return findings;
    }

    for pane in unreachable(&list, &offered) {
        findings.push(Finding {
            file: PANES.into(),
            line: 1,
            what: format!(
                "`{pane}` is a pane nobody can click for: no screen calls open('{pane}') or \
                 show('{pane}'). The command palette does not count — that is the state the \
                 browser pane shipped in. Offer it somewhere, or say in xtask/src/reachable.rs \
                 what opens it"
            ),
        });
    }
    findings
}

#[cfg(test)]
#[path = "reachable_tests.rs"]
mod tests;
