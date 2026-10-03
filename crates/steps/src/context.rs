//! The context a command step is given, and the only way it is given.
//!
//! **Through the environment, never interpolated into the command string.** A
//! branch called `x; rm -rf /` becomes the value of a variable rather than
//! shell syntax, and that removes an entire class of injection instead of
//! trying to escape its way out of one.
//!
//! The keys are a closed set. A manifest naming one that does not exist is
//! refused when it loads, by name, rather than expanding to nothing at the
//! moment it matters.

/// Every key a manifest may name.
pub const CONTEXT_KEYS: &[&str] = &[
    "project",
    "projectPath",
    "worktreePath",
    "branch",
    "baseRef",
    "card",
    "cardTitle",
    "reportDir",
];

/// The values behind those keys, for one run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    pub project: String,
    pub project_path: String,
    pub worktree_path: String,
    /// The branch the checkout is on. Empty when the step is running in the
    /// project itself, which is on whatever branch the person left it on.
    pub branch: String,
    /// The commit the card's work started from, as a SHA. A different thing
    /// from `branch`, which is where the work is going.
    pub base_ref: String,
    pub card: String,
    pub card_title: String,
    /// This run's empty folder for test reports.
    pub report_dir: String,
    /// The variables the project's worktree preparation shares with every
    /// command in a checkout (`CARGO_TARGET_DIR`, say). Never a `DEVPIT_` key:
    /// those are devpit's to say.
    pub shared: Vec<(String, String)>,
}

impl Context {
    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "project" => Some(&self.project),
            "projectPath" => Some(&self.project_path),
            "worktreePath" => Some(&self.worktree_path),
            "branch" => Some(&self.branch),
            "baseRef" => Some(&self.base_ref),
            "card" => Some(&self.card),
            "cardTitle" => Some(&self.card_title),
            "reportDir" => Some(&self.report_dir),
            _ => None,
        }
    }

    /// The environment a command is run with: one variable per key.
    ///
    /// `DEVPIT_` prefixed so a command can tell them from its own, and
    /// upper-snake because that is what a shell script expects to read.
    pub fn environment(&self) -> Vec<(String, String)> {
        // Shared first: a later variable of the same name wins, and the
        // context's own are the ones that must.
        self.shared
            .iter()
            .filter(|(name, _)| !name.starts_with("DEVPIT_"))
            .cloned()
            .chain(CONTEXT_KEYS.iter().map(|key| {
                (
                    format!("DEVPIT_{}", screaming_snake(key)),
                    self.get(key).unwrap_or_default().to_owned(),
                )
            }))
            .collect()
    }
}

fn screaming_snake(key: &str) -> String {
    let mut out = String::new();
    for ch in key.chars() {
        if ch.is_ascii_uppercase() && !out.is_empty() {
            out.push('_');
        }
        out.push(ch.to_ascii_uppercase());
    }
    out
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
