//! Devflow's skills over MCP, for hosts that do not speak the Skills
//! extension (SEP-2640) yet: each one a `skill://` resource carrying the
//! extension's entry, `devpit_devflow_skill` returning one, and a prompt per
//! stage. Only where Devflow is on, and resolved by the app on every ask.
//!
//! When hosts declare `io.modelcontextprotocol/skills`, `skills/list` and
//! `skills/get` answer with the same entries beside these, and
//! `devpit_devflow_skill` is deprecated: none of this goes away first.

use serde_json::{json, Value};

use crate::devflow::Offer;

type Ask<'a> = &'a dyn Fn(&str, Value) -> Result<Value, String>;

/// The tool that returns a skill.
pub(crate) const TOOL: &str = "devpit_devflow_skill";

/// Every skill a project can be given, as the tool's choices.
pub const NAMES: [&str; 7] = [
    "devflow-briefing",
    "devflow-context",
    "devflow-plan",
    "devflow-execution",
    "devflow-review",
    "devflow-completion",
    "devflow-setup",
];

const MIME: &str = "text/markdown";

/// Where a listed resource carries its entry, under the extension's prefix.
const ENTRY: &str = "io.modelcontextprotocol.skills/skill";

/// The app's answer for each skill offered here; none where Devflow is off.
fn listed(offer: &Offer, ask: Ask) -> Vec<Value> {
    if !matches!(offer, Offer::Active(_)) {
        return Vec::new();
    }
    ask("devflow_skills", json!({}))
        .ok()
        .and_then(|answer| answer.get("skills").and_then(Value::as_array).cloned())
        .unwrap_or_default()
}

/// One skill with its text, asked only where Devflow is on.
fn one(params: Value, offer: &Offer, ask: Ask) -> Result<Value, String> {
    if !matches!(offer, Offer::Active(_)) {
        return Err("Devflow is not on in this project".to_owned());
    }
    ask("devflow_skill", params)
}

/// `resources/list`'s share: each skill, its entry in `_meta`.
pub(crate) fn resources(offer: &Offer, ask: Ask) -> Vec<Value> {
    listed(offer, ask)
        .iter()
        .filter_map(|listed| {
            let skill = listed.get("skill")?;
            Some(json!({
                "uri": skill.get("uri")?,
                "name": listed.get("name")?,
                "description": skill["frontmatter"]["description"],
                "mimeType": MIME,
                "size": skill["resources"][0].get("size")?,
                "_meta": { ENTRY: skill },
            }))
        })
        .collect()
}

/// `resources/read` of a skill's URI, or `None` for a URI that is not one.
pub(crate) fn read(uri: &str, offer: &Offer, ask: Ask) -> Option<Result<Value, String>> {
    uri.starts_with("skill://").then(|| {
        let skill = one(json!({ "uri": uri }), offer, ask)?;
        Ok(json!({ "contents": [{ "uri": uri, "mimeType": MIME, "text": skill["text"] }] }))
    })
}

/// `prompts/list`: one per stage switched on, `/devpit:devflow-<stage>` in
/// Claude Code.
pub(crate) fn prompts(offer: &Offer, ask: Ask) -> Value {
    let prompts: Vec<Value> = listed(offer, ask)
        .iter()
        .filter(|listed| !listed["stage"].is_null())
        .map(|listed| {
            json!({
                "name": listed["name"],
                "description": listed["skill"]["frontmatter"]["description"],
            })
        })
        .collect();
    json!({ "prompts": prompts })
}

/// `prompts/get`: the stage's skill, put to the agent as the person's message.
pub(crate) fn prompt(params: &Value, offer: &Offer, ask: Ask) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let skill = one(json!({ "name": name }), offer, ask)?;
    if skill["stage"].is_null() {
        return Err(format!("devpit has no prompt `{name}`"));
    }
    Ok(json!({
        "description": skill["skill"]["frontmatter"]["description"],
        "messages": [{ "role": "user", "content": { "type": "text", "text": skill["text"] } }],
    }))
}

/// What the tool says: the SKILL.md as it is, then where it came from.
pub(crate) fn said(answer: &Value) -> Value {
    let from = match answer["file"].as_str() {
        Some(file) => format!(
            "the {} layer, {file}",
            answer["layer"].as_str().unwrap_or("")
        ),
        None => "devpit's built-in skills".to_owned(),
    };
    let about = format!(
        "{} is from {from}; {}.",
        answer["name"].as_str().unwrap_or("This skill"),
        answer["skill"]["resources"][0]["digest"]
            .as_str()
            .unwrap_or("no digest"),
    );
    json!({ "content": [
        { "type": "text", "text": answer["text"].as_str().unwrap_or_default() },
        { "type": "text", "text": about },
    ], "isError": false })
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
