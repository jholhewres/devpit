//! What a command step declares, and what is refused at load time.
//!
//! A manifest that names a context key which does not exist is refused by
//! name, when it loads. Left to run time, the same mistake expands to an empty
//! string at the exact moment it matters least to discover it.

use serde::{Deserialize, Serialize};

use crate::context::CONTEXT_KEYS;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ManifestError {
    #[error("`{0}` is not a context key — the ones that exist are: {1}")]
    UnknownKey(String, String),

    #[error("a command step has no command")]
    NoCommand,

    #[error("this step's config is not readable: {0}")]
    Unreadable(String),
}

/// A command step, as it is stored.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub command: String,
    /// Seconds. Absent means the command may take as long as it takes, which
    /// is a choice worth making deliberately rather than falling into.
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

/// Reads a step's config and refuses what would fail later.
pub fn validate(config: &str) -> Result<Manifest, ManifestError> {
    let manifest: Manifest =
        serde_json::from_str(config).map_err(|err| ManifestError::Unreadable(err.to_string()))?;

    if manifest.command.trim().is_empty() {
        return Err(ManifestError::NoCommand);
    }

    for key in placeholders(&manifest.command) {
        if !CONTEXT_KEYS.contains(&key) {
            return Err(ManifestError::UnknownKey(
                key.to_owned(),
                CONTEXT_KEYS.join(", "),
            ));
        }
    }

    Ok(manifest)
}

/// Every `{{key}}` in the text, in order.
///
/// A `{{` with no closing `}}` is not a placeholder — it is ordinary text that
/// happens to start with two braces — and the scan stops there without
/// complaining.
fn placeholders(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        let after = &rest[start + 2..];
        match after.find("}}") {
            Some(end) => {
                found.push(after[..end].trim());
                rest = &after[end + 2..];
            }
            None => break,
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_command_loads() {
        let manifest = validate(r#"{"command":"make test"}"#).expect("valid");
        assert_eq!(manifest.command, "make test");
        assert_eq!(manifest.timeout_seconds, None);
    }

    #[test]
    fn a_timeout_is_read_when_it_is_declared() {
        let manifest = validate(r#"{"command":"make test","timeoutSeconds":600}"#).expect("valid");
        assert_eq!(manifest.timeout_seconds, Some(600));
    }

    /// The failure this catches: a key that would expand to nothing at the
    /// moment it matters least to find out.
    #[test]
    fn a_key_that_does_not_exist_is_refused_by_name() {
        let error = validate(r#"{"command":"deploy {{enviroment}}"}"#).expect_err("should refuse");
        assert_eq!(
            error,
            ManifestError::UnknownKey("enviroment".to_owned(), CONTEXT_KEYS.join(", "))
        );
    }

    #[test]
    fn a_key_that_exists_is_accepted() {
        assert!(validate(r#"{"command":"deploy {{branch}}"}"#).is_ok());
    }

    #[test]
    fn a_step_with_no_command_is_refused() {
        assert_eq!(
            validate(r#"{"command":"  "}"#),
            Err(ManifestError::NoCommand)
        );
    }

    #[test]
    fn config_that_is_not_json_says_so() {
        assert!(matches!(
            validate("make test"),
            Err(ManifestError::Unreadable(_))
        ));
    }

    /// Two braces in a shell command are not always a placeholder.
    #[test]
    fn an_unclosed_brace_is_ordinary_text() {
        assert!(validate(r#"{"command":"awk '{{print $1}'"}"#).is_ok());
    }
}
