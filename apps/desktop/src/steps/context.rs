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
    /// The name of the branch the card's checkout is on. Absent when the step
    /// has no checkout of its own: it runs in the project, on whatever branch
    /// the person left it on, which is not the card's.
    pub branch: Option<String>,
    /// The commit the card's work started from, as a SHA — a different thing
    /// from the branch, and for a long time the value `branch` was given.
    pub base_ref: Option<String>,
    pub project: Option<String>,
    pub worktree_path: Option<String>,
    pub project_path: Option<String>,
}

/// A checkout of the card's own, and the branch git says it is on.
pub struct Checkout<'a> {
    pub path: &'a std::path::Path,
    pub branch: &'a str,
}

/// What a step is told, from the card and where it will run.
///
/// Pure, and told the branch rather than reading it: the name comes from git,
/// and the rule this settles — which of these is the branch and which is the
/// base commit — has to be answerable without a repository.
///
/// Both runners build their context from this. They each had their own copy,
/// and both copies put the base SHA in `branch`.
pub fn context_of(
    card: &devpit_core::CardRow,
    checkout: Option<Checkout<'_>>,
    project: Option<&str>,
) -> Context {
    Context {
        card: card.id.clone(),
        card_title: card.title.clone(),
        card_body: card.body.clone(),
        // Only from a checkout: without one there is no branch that belongs to
        // this card, and naming the project's would be a claim about somebody
        // else's work.
        branch: checkout
            .as_ref()
            .map(|open| open.branch.to_owned())
            .filter(|name| !name.is_empty()),
        base_ref: card.base_ref.clone(),
        project: project.map(name_of),
        worktree_path: checkout.map(|open| open.path.display().to_string()),
        project_path: project.map(str::to_owned),
    }
}

/// A project's name is its folder's, which is what a person calls it.
fn name_of(path: &str) -> String {
    std::path::Path::new(path)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_owned())
}

impl Context {
    /// The same facts as a command step's context.
    ///
    /// Everything absent becomes empty there, because a `{{key}}` in a command
    /// line always expands to something — while a variable a turn never gets
    /// is a key that says "there is none of this".
    pub fn for_a_command(&self) -> devpit_steps::Context {
        devpit_steps::Context {
            project: self.project.clone().unwrap_or_default(),
            project_path: self.project_path.clone().unwrap_or_default(),
            worktree_path: self.worktree_path.clone().unwrap_or_default(),
            branch: self.branch.clone().unwrap_or_default(),
            base_ref: self.base_ref.clone().unwrap_or_default(),
            card: self.card.clone(),
            card_title: self.card_title.clone(),
        }
    }
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
                "baseRef" => context.base_ref.clone(),
                "project" => context.project.clone(),
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
    "baseRef",
    "project",
    "worktreePath",
    "projectPath",
];

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
