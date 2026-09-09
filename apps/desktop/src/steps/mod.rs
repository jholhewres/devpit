//! The three kinds of step, one module each.
//!
//! They behave in opposite ways and the difference is the product: only
//! `session` takes the terminal, only `command` has an exit code, only `agent`
//! reports a cost. Splitting them keeps each one readable on its own — the
//! size ratchet asked for this, and it was right.

pub mod agent;
pub mod command;
pub mod context;
pub mod recipe;
pub mod session;

use std::path::PathBuf;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
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

/// Writes the hook settings once and hands back their path.
///
/// Next to the state rather than in a temp file: a turn that outlives the app
/// still has a file to read, and a path that changes every run would be a new
/// file on disk for every card moved.
fn hook_settings() -> Option<String> {
    let root = Store::root().ok()?;
    let endpoint = devpit_agentcli::endpoint_file(&root);
    let path = root.join("hooks.json");
    let wanted = devpit_agentcli::settings_json(&endpoint);

    // Rewritten only when it differs, so a turn does not touch the disk for
    // nothing.
    if std::fs::read_to_string(&path).ok().as_deref() != Some(wanted.as_str()) {
        std::fs::create_dir_all(&root).ok()?;
        std::fs::write(&path, &wanted).ok()?;
    }
    Some(path.display().to_string())
}

/// Where the agents on this machine live.
pub fn agents_dir() -> Result<PathBuf, RpcError> {
    Ok(Store::root()
        .map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?
        .join("agents"))
}

/// `agents.list` — the agents on this machine, and the files that would not
/// load.
///
/// The step picker needs the names: a step stores an agent by the name in its
/// frontmatter, and a free-text field over twenty-eight files on disk is a
/// typo waiting to fail at the moment the step runs.
#[tauri::command]
#[specta::specta]
pub fn agents_list() -> Result<devpit_rpc::Agents, RpcError> {
    let dir = agents_dir()?;
    let catalogue = devpit_agentcli::read_agents(&dir);
    Ok(devpit_rpc::Agents {
        agents: catalogue
            .agents
            .into_iter()
            .map(|agent| devpit_rpc::Agent {
                name: agent.name,
                description: agent.description,
                model: agent.model,
            })
            .collect(),
        rejected: catalogue
            .rejected
            .into_iter()
            .map(|one| devpit_rpc::RejectedAgent {
                file: one.file,
                reason: one.reason,
            })
            .collect(),
        directory: dir.display().to_string(),
    })
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
