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
    /// Which CLI account runs it, as a profile id. A different thing from
    /// `agent`, which is a subagent from somebody's frontmatter.
    profile: Option<String>,
    #[serde(alias = "capUsd")]
    budget_usd: Option<f64>,
    skills: Vec<String>,
    inject: Vec<String>,
}

/// Why this step cannot be saved, or nothing when it can.
///
/// `agents`, `installed` and `profiles` are passed in rather than read here so
/// the rule is callable from a test without a machine that happens to have
/// them.
pub fn refuse(
    kind: StepKind,
    config: &str,
    agents: &[String],
    installed: &[String],
    profiles: &[String],
) -> Option<String> {
    match kind {
        // Each kind is read here by whatever will run it, so a step that saves
        // is a step that runs. What used to happen instead: anything that was
        // not JSON parsed into nothing, was accepted, and failed on the lane.
        StepKind::Command => {
            return devpit_steps::validate(config)
                .err()
                .map(|err| err.to_string())
        }
        StepKind::Session => return super::session::readable(config).err(),
        StepKind::Agent => {}
    }
    let declared: Declared = match serde_json::from_str(config) {
        Ok(declared) => declared,
        Err(err) => return Some(format!("this step's config is not readable: {err}")),
    };

    if declared.budget_usd.is_none() {
        return Some("this step declares no spending cap, so it would never run".to_owned());
    }
    if let Some(name) = &declared.agent {
        if !agents.contains(name) {
            return Some(format!("no agent named `{name}` on this machine"));
        }
    }
    if let Some(id) = &declared.profile {
        if !profiles.contains(id) {
            return Some("this step names a profile that does not exist".to_owned());
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
