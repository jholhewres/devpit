//! The skills of an active project, for the MCP server: their entries to
//! list, and one skill's text to read, to call or to start a stage with.
//!
//! Resolved in the project's root, never a card's worktree, so every card
//! of a project follows the same flow.

use std::path::{Path, PathBuf};

use devpit_devflow::layers::Team;
use devpit_devflow::skills;
use serde_json::{json, Value};

pub(crate) fn respond(
    method: &str,
    root: &Path,
    stages: &[&str],
    params: &Value,
) -> Result<Value, String> {
    let personal = personal()?;
    // A team layer that applies replaces a built-in skill by its name too.
    let teams = crate::devflow::layered(root)
        .map(|layered| layered.applied)
        .unwrap_or_default();
    match method {
        "devflow_skills" => Ok(listed(root, &personal, &teams, stages)),
        _ => one(root, &personal, &teams, params),
    }
}

/// The person's own skills, under this devpit's home.
pub(crate) fn personal() -> Result<PathBuf, String> {
    let root = devpit_core::Store::root().map_err(|err| err.to_string())?;
    Ok(devpit_core::home::devflow_skills(&root))
}

/// Each skill offered, without its text. One whose file is refused is named
/// with why, not left out unsaid.
fn listed(root: &Path, personal: &Path, teams: &[Team], stages: &[&str]) -> Value {
    let (mut skills, mut refused) = (Vec::new(), Vec::new());
    for name in skills::offered(stages) {
        match skills::resolve_with(root, personal, teams, name) {
            Ok(skill) => skills.push(skill.to_json()),
            Err(why) => refused.push(json!({ "name": name, "why": why })),
        }
    }
    json!({ "skills": skills, "refused": refused })
}

/// One skill, by `name` or `uri`, with its text: any stage's, asked by
/// name, so a stage can be run on demand where devflow.yaml leaves it off.
fn one(root: &Path, personal: &Path, teams: &[Team], params: &Value) -> Result<Value, String> {
    let asked = ["name", "uri"]
        .iter()
        .find_map(|key| params.get(key).and_then(Value::as_str))
        .ok_or("pass the skill's `name`, such as devflow-plan")?;
    let name = skills::named(asked).ok_or_else(|| {
        format!(
            "`{asked}` is not a Devflow skill: {}",
            skills::names().collect::<Vec<_>>().join(", ")
        )
    })?;
    let skill = skills::resolve_with(root, personal, teams, name)?;
    let mut said = skill.to_json();
    said["text"] = json!(skill.text);
    Ok(said)
}

#[cfg(test)]
#[path = "devflow_skills_tests.rs"]
mod tests;
