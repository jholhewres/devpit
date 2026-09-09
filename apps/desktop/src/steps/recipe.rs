//! What a step declares, checked when it is saved.
//!
//! Saving is when a person is still looking at what they typed. Checking at
//! run time instead means a card lands on a column and stops, and the typo is
//! two screens away from the message.

use devpit_rpc::StepKind;
use serde::Deserialize;

use super::context::CONTEXT_KEYS;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Declared {
    agent: Option<String>,
    #[serde(alias = "capUsd")]
    budget_usd: Option<f64>,
    skills: Vec<String>,
    inject: Vec<String>,
}

/// Why this step cannot be saved, or nothing when it can.
///
/// `agents` and `installed` are passed in rather than read here so the rule is
/// callable from a test without a machine that happens to have them.
pub fn refuse(
    kind: StepKind,
    config: &str,
    agents: &[String],
    installed: &[String],
) -> Option<String> {
    if kind != StepKind::Agent {
        return None;
    }
    let declared: Declared = serde_json::from_str(config).ok()?;

    if declared.budget_usd.is_none() {
        return Some("this step declares no spending cap, so it would never run".to_owned());
    }
    if let Some(name) = &declared.agent {
        if !agents.contains(name) {
            return Some(format!("no agent named `{name}` on this machine"));
        }
    }
    let missing = devpit_agentcli::skills::missing(&declared.skills, installed);
    if let Some(name) = missing.first() {
        return Some(format!("no skill named `{name}` on this machine"));
    }
    if let Some(key) = declared
        .inject
        .iter()
        .find(|key| !CONTEXT_KEYS.contains(&key.as_str()))
    {
        return Some(format!(
            "there is no context called `{key}` — try one of: {}",
            CONTEXT_KEYS.join(", ")
        ));
    }
    None
}

#[cfg(test)]
#[path = "recipe_tests.rs"]
mod tests;
