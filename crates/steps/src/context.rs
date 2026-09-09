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
    "card",
    "cardTitle",
];

/// The values behind those keys, for one run.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Context {
    pub project: String,
    pub project_path: String,
    pub worktree_path: String,
    pub branch: String,
    pub card: String,
    pub card_title: String,
}

impl Context {
    pub fn get(&self, key: &str) -> Option<&str> {
        match key {
            "project" => Some(&self.project),
            "projectPath" => Some(&self.project_path),
            "worktreePath" => Some(&self.worktree_path),
            "branch" => Some(&self.branch),
            "card" => Some(&self.card),
            "cardTitle" => Some(&self.card_title),
            _ => None,
        }
    }

    /// The environment a command is run with: one variable per key.
    ///
    /// `DEVPIT_` prefixed so a command can tell them from its own, and
    /// upper-snake because that is what a shell script expects to read.
    pub fn environment(&self) -> Vec<(String, String)> {
        CONTEXT_KEYS
            .iter()
            .map(|key| {
                (
                    format!("DEVPIT_{}", screaming_snake(key)),
                    self.get(key).unwrap_or_default().to_owned(),
                )
            })
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
mod tests {
    use super::*;

    fn sample() -> Context {
        Context {
            project: "devpit".to_owned(),
            project_path: "/home/x/devpit".to_owned(),
            worktree_path: "/home/x/devpit".to_owned(),
            branch: "main".to_owned(),
            card: "card_1".to_owned(),
            card_title: "Ship it".to_owned(),
        }
    }

    #[test]
    fn every_key_has_a_value_behind_it() {
        let context = sample();
        for key in CONTEXT_KEYS {
            assert!(context.get(key).is_some(), "no value for {key}");
        }
    }

    #[test]
    fn a_key_that_does_not_exist_has_no_value() {
        assert_eq!(sample().get("secrets"), None);
    }

    #[test]
    fn the_environment_names_are_what_a_shell_script_expects() {
        let environment = sample().environment();
        let names: Vec<&str> = environment.iter().map(|(k, _)| k.as_str()).collect();
        assert!(names.contains(&"DEVPIT_PROJECT_PATH"));
        assert!(names.contains(&"DEVPIT_CARD_TITLE"));
        assert!(names.contains(&"DEVPIT_BRANCH"));
        assert_eq!(names.len(), CONTEXT_KEYS.len());
    }

    /// The whole reason the context travels this way.
    #[test]
    fn a_branch_name_full_of_shell_syntax_is_only_ever_a_value() {
        let context = Context {
            branch: "x; rm -rf /tmp/proof".to_owned(),
            ..sample()
        };
        let environment = context.environment();
        let (_, branch) = environment
            .iter()
            .find(|(k, _)| k == "DEVPIT_BRANCH")
            .expect("no branch in the environment");
        assert_eq!(branch, "x; rm -rf /tmp/proof");
    }
}
