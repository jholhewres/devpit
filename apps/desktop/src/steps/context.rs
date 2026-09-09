//! What a step is told about the card it runs on.
//!
//! Environment variables and never text pasted into a command: a branch name
//! is not a safe string, and the difference between a value and a command is
//! the whole of this module.

/// What a step may be told about the card it is running on.
#[derive(Debug, Default, Clone)]
pub struct Context {
    pub card: String,
    pub card_title: String,
    pub card_body: String,
    pub branch: Option<String>,
    pub worktree_path: Option<String>,
    pub project_path: Option<String>,
}

/// The context keys a step declared, as environment variables.
///
/// Variables and never interpolation: a branch named `fix;rm -rf /` becomes a
/// value here, and a value cannot be shell syntax. A key nothing answers is
/// left out rather than sent empty — an empty variable reads as "there is none
/// of this", which is a different claim from "we did not look".
pub fn injected(context: &Context, keys: &[String]) -> Vec<(String, String)> {
    keys.iter()
        .filter_map(|key| {
            let value = match key.as_str() {
                "card" => Some(context.card.clone()),
                "cardTitle" => Some(context.card_title.clone()),
                "cardBody" => Some(context.card_body.clone()),
                "branch" => context.branch.clone(),
                "worktreePath" => context.worktree_path.clone(),
                "projectPath" => context.project_path.clone(),
                _ => None,
            }?;
            Some((format!("DEVPIT_{}", shout(key)), value))
        })
        .collect()
}

/// `cardTitle` as `CARD_TITLE`. Environment variables are shouted by
/// convention, and a lowercase one reads as a mistake.
fn shout(key: &str) -> String {
    let mut out = String::new();
    for ch in key.chars() {
        if ch.is_ascii_uppercase() && !out.is_empty() {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    out
}

/// The keys a step may inject. Named here so saving a column can refuse one
/// that does not exist, by name, instead of silently sending nothing.
pub const CONTEXT_KEYS: &[&str] = &[
    "card",
    "cardTitle",
    "cardBody",
    "branch",
    "worktreePath",
    "projectPath",
];

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
