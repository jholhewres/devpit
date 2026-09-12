//! What a step's answer said about itself.
//!
//! Three questions about one piece of config, and they belong together
//! because they are easy to get subtly wrong apart: whether a step has an
//! opinion to give at all, whether the answer carried it, and whether it
//! asked for the card to go back.
//!
//! The one that matters: **silence is not approval.** A run that ends `ok`
//! with an answer missing the declared field is a run with no opinion, and
//! reading that as a pass advances somebody's card on a judgment nobody made.

use serde::Deserialize;

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

/// Whether this step declares a verdict at all.
///
/// A step with no `verdictField` has no opinion to give, so its answer is not
/// an approval and never was — it just finished.
pub fn declares_a_verdict(config: &str) -> bool {
    serde_json::from_str::<Verdict>(config)
        .map(|declared| declared.verdict_field.is_some())
        .unwrap_or(false)
}

/// Whether the answer actually carried the verdict the step asked for.
///
/// Present and not the send-back value. Absent is silence, and silence is not
/// approval: a run that ends `ok` with an answer missing the field is a run
/// with no opinion, and reading that as a pass advances a card on a judgment
/// nobody made.
pub fn verdict_given(config: &str, answer: &str) -> bool {
    let Ok(declared) = serde_json::from_str::<Verdict>(config) else {
        return false;
    };
    let Some(field) = declared.verdict_field else {
        return false;
    };
    let Ok(answer) = serde_json::from_str::<serde_json::Value>(answer) else {
        return false;
    };
    let Some(said) = answer.get(&field).and_then(|value| value.as_str()) else {
        return false;
    };
    declared.sends_back_when.as_deref() != Some(said)
}

#[cfg(test)]
#[path = "verdict_tests.rs"]
mod tests;
