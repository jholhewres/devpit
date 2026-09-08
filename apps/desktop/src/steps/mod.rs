//! The three kinds of step, one module each.
//!
//! They behave in opposite ways and the difference is the product: only
//! `session` takes the terminal, only `command` has an exit code, only `agent`
//! reports a cost. Splitting them keeps each one readable on its own — the
//! size ratchet asked for this, and it was right.

pub mod agent;
pub mod command;
pub mod session;

use std::path::PathBuf;

use quockpit_core::Store;
use quockpit_rpc::{ErrorCode, RpcError};
use serde::Deserialize;

/// What a step produced, whichever kind it was.
pub struct Finished {
    pub ok: bool,
    pub output: String,
    pub cost_usd: f64,
    pub duration_ms: i64,
    /// Only a command has one. An agent turn reports cost, not an exit code.
    pub exit_code: Option<i32>,
}

/// The verdict fields an agent step may declare.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Verdict {
    verdict_field: Option<String>,
    sends_back_when: Option<String>,
}

/// Whether an answer asked for the card to go back, and why.
///
/// The only automatic transition in the product. It exists because a review
/// that says "revise" and then leaves the card sitting in the reviewed column
/// is a review nobody acts on.
pub fn sends_back(config: &str, answer: &str) -> Option<String> {
    let config: Verdict = serde_json::from_str(config).ok()?;
    let field = config.verdict_field?;
    let back = config.sends_back_when?;
    let answer: serde_json::Value = serde_json::from_str(answer).ok()?;
    let verdict = answer.get(&field)?.as_str()?;
    (verdict == back).then(|| format!("{field}: {verdict}"))
}

/// A session id shaped like the UUID the CLI expects, derived from the card so
/// the same card keeps the same id across restarts.
fn uuid_like(card_id: &str) -> String {
    let digest: Vec<String> = card_id
        .bytes()
        .cycle()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect();
    let hex = digest.concat();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// A branch-safe name from a card title.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(40).collect()
}

/// Where the agents on this machine live.
pub fn agents_dir() -> Result<PathBuf, RpcError> {
    Ok(Store::root()
        .map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?
        .join("agents"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const REVIEW: &str =
        r#"{"prompt":"review it","verdictField":"verdict","sendsBackWhen":"revise"}"#;

    /// The only automatic transition in the product, and it only goes back.
    #[test]
    fn a_verdict_of_revise_sends_the_card_back() {
        let why = sends_back(REVIEW, r#"{"verdict":"revise","findings":["no tests"]}"#)
            .expect("should send back");
        assert!(why.contains("revise"), "{why}");
    }

    #[test]
    fn an_approving_verdict_leaves_the_card_where_it_is() {
        assert_eq!(sends_back(REVIEW, r#"{"verdict":"approved"}"#), None);
    }

    /// A step that declares no verdict never moves a card on its own.
    #[test]
    fn a_step_without_a_verdict_never_sends_anything_back() {
        assert_eq!(
            sends_back(r#"{"prompt":"refine it"}"#, r#"{"verdict":"revise"}"#),
            None
        );
    }

    /// Prose where a verdict was expected is not a verdict. Reading one out of
    /// it would move cards on a guess.
    #[test]
    fn an_answer_that_is_not_json_moves_nothing() {
        assert_eq!(sends_back(REVIEW, "I think you should revise this"), None);
    }

    /// A branch name has to survive a title with punctuation in it.
    #[test]
    fn a_card_title_becomes_a_branch_safe_name() {
        assert_eq!(slug("Fix the OAuth flow!"), "fix-the-oauth-flow");
        assert_eq!(slug("  spaces  everywhere  "), "spaces-everywhere");
        assert!(slug(&"x".repeat(80)).len() <= 40);
    }

    /// The same card keeps the same session id across restarts, which is what
    /// makes its transcript findable later.
    #[test]
    fn a_card_always_gets_the_same_session_id() {
        let first = uuid_like("card_01HX");
        assert_eq!(first, uuid_like("card_01HX"));
        assert_ne!(first, uuid_like("card_01HY"));
        assert_eq!(first.len(), 36, "{first} is not shaped like a uuid");
        assert_eq!(first.matches('-').count(), 4);
    }
}
