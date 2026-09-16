//! A command the window never calls.
//!
//! The contract is generated from a list, and a command on that list costs
//! nothing to leave there — it compiles, it is typed, and it appears in
//! `bindings.ts` beside the ones the window uses. So nine of them sat there,
//! and reading the contract to learn what the app does told you about screens
//! that were never built.
//!
//! The exceptions are real and few: a command whose only job is to make specta
//! write down a type the window reads off an event. They are listed by name,
//! with the reason, in `uncalled-commands.txt`.

use std::path::{Path, PathBuf};

use crate::Finding;

/// The commands allowed to have no caller. One `name reason` per line.
pub(crate) const ALLOWED: &str = include_str!("../uncalled-commands.txt");

/// Where the commands are registered.
const CONTRACT: &str = "apps/desktop/src/contract_list.rs";
const HANDLER: &str = "apps/desktop/src/handler.rs";
const LISTS: [&str; 2] = [CONTRACT, HANDLER];

/// Every command is called from the window, or listed with a reason.
pub fn a_command_has_a_caller(root: &Path) -> Vec<Finding> {
    uncalled_in(root, ALLOWED)
}

/// Every command in the contract is one the app actually answers.
///
/// The contract is what the window's client is generated from, so a command
/// there and not in the invoke handler is a button that fails at run time with
/// "command not found" — which is how the file tree shipped with create,
/// rename, move and delete all broken.
pub fn the_app_answers_what_the_contract_offers(root: &Path) -> Vec<Finding> {
    let answered = registered_in(root, HANDLER);
    let mut findings = Vec::new();
    for (command, line) in registered_in(root, CONTRACT) {
        if !answered.iter().any(|(name, _)| name == &command) {
            findings.push(Finding {
                file: PathBuf::from(CONTRACT),
                line,
                what: format!(
                    "`{command}` is in the contract and not in the invoke handler — \
                     the window can call it and the app cannot answer"
                ),
            });
        }
    }
    findings
}

/// The commands one list registers, with the line each is on.
fn registered_in(root: &Path, list: &str) -> Vec<(String, usize)> {
    let Ok(text) = std::fs::read_to_string(root.join(list)) else {
        return Vec::new();
    };
    text.lines()
        .enumerate()
        .filter_map(|(index, line)| registered(line).map(|name| (name, index + 1)))
        .collect()
}

fn uncalled_in(root: &Path, allowed: &str) -> Vec<Finding> {
    let allowed = names(allowed);
    let calls = window_text(&root.join("web/src"));
    let mut findings = Vec::new();
    let mut seen: Vec<String> = Vec::new();

    for list in LISTS {
        let Ok(text) = std::fs::read_to_string(root.join(list)) else {
            continue;
        };
        for (index, line) in text.lines().enumerate() {
            let Some(command) = registered(line) else {
                continue;
            };
            if seen.contains(&command) {
                continue;
            }
            seen.push(command.clone());
            if called(&calls, &command) || allowed.contains(&command) {
                continue;
            }
            findings.push(Finding {
                file: PathBuf::from(list),
                line: index + 1,
                what: format!(
                    "`{command}` is in the contract and nothing in web/src calls it — \
                     remove it, or give it a reason in xtask/uncalled-commands.txt"
                ),
            });
        }
    }

    // The other direction: a reason nobody needs any more.
    for name in &allowed {
        if !seen.contains(name) {
            findings.push(Finding {
                file: PathBuf::from("xtask/uncalled-commands.txt"),
                line: 1,
                what: format!("`{name}` is not a command any more — drop the entry"),
            });
        } else if called(&calls, name) {
            findings.push(Finding {
                file: PathBuf::from("xtask/uncalled-commands.txt"),
                line: 1,
                what: format!("`{name}` has a caller now — drop the entry"),
            });
        }
    }

    findings
}

/// The command a registration line names, as it is written in Rust.
fn registered(line: &str) -> Option<String> {
    let line = line.trim();
    if line.starts_with("//") {
        return None;
    }
    let name = line.strip_suffix(',')?;
    let (module, command) = name.rsplit_once("::")?;
    let ok = |text: &str| {
        !text.is_empty()
            && text
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
    };
    (ok(module) && ok(command)).then(|| command.to_owned())
}

/// Whether the window calls it, by either name it can be called by.
fn called(text: &str, command: &str) -> bool {
    text.contains(&format!("commands.{}", camel(command)))
        || text.contains(&format!("invoke(\"{command}\""))
        || text.contains(&format!("invoke('{command}'"))
}

/// `card_board` as `cardBoard`, which is how the generated client spells it.
fn camel(command: &str) -> String {
    let mut out = String::new();
    let mut shout = false;
    for ch in command.chars() {
        if ch == '_' {
            shout = true;
        } else if shout {
            out.push(ch.to_ascii_uppercase());
            shout = false;
        } else {
            out.push(ch);
        }
    }
    out
}

/// Everything the window is written in, minus what is generated from the
/// contract — `bindings.ts` names every command, including the dead ones.
fn window_text(dir: &Path) -> String {
    let mut text = String::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return text;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "gen") {
                continue;
            }
            text.push_str(&window_text(&path));
        } else if path
            .extension()
            .is_some_and(|ext| ext == "ts" || ext == "tsx")
        {
            text.push_str(&std::fs::read_to_string(&path).unwrap_or_default());
            text.push('\n');
        }
    }
    text
}

/// The names in the list, ignoring comments and the reasons.
fn names(list: &str) -> Vec<String> {
    list.lines()
        .map(|line| line.split('#').next().unwrap_or(line).trim())
        .filter(|line| !line.is_empty())
        .filter_map(|line| line.split_whitespace().next().map(str::to_owned))
        .collect()
}

#[cfg(test)]
#[path = "uncalled_tests.rs"]
mod tests;
