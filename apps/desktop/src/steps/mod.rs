//! The three kinds of step, one module each.
//!
//! They behave in opposite ways and the difference is the product: only
//! `session` takes the terminal, only `command` has an exit code, only `agent`
//! reports a cost. Splitting them keeps each one readable on its own — the
//! size ratchet asked for this, and it was right.

pub mod agent;
pub mod agent_config;
pub mod command;
pub mod context;
pub mod recipe;
pub mod session;
pub mod verdict;

use std::path::PathBuf;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};

/// What a step produced, whichever kind it was.
pub struct Finished {
    pub ok: bool,
    pub output: String,
    pub cost_usd: f64,
    pub duration_ms: i64,
    /// Only a command has one. An agent turn reports cost, not an exit code.
    pub exit_code: Option<i32>,
}

/// A new session id for one run, shaped like the version-4 UUID the CLI takes.
///
/// New for every run rather than derived from the card: two runs of one card
/// are two sessions, and one id for both would put the second transcript on
/// top of the first.
pub(crate) fn fresh_session_id() -> String {
    let random = ulid::Ulid::generate().0;
    let bits = (random & !(0xf_u128 << 76) & !(0x3_u128 << 62)) | (0x4 << 76) | (0x2 << 62);
    let hex = format!("{bits:032x}");
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
    let wanted = devpit_agentcli::settings_json(&endpoint, &devpit_agentcli::auth_file(&root));

    // Rewritten only when it differs, so a turn does not touch the disk for
    // nothing.
    if std::fs::read_to_string(&path).ok().as_deref() != Some(wanted.as_str()) {
        std::fs::create_dir_all(&root).ok()?;
        // Private, like the other writer of this file: these are the commands
        // an agent runs on every hook.
        devpit_core::home::write_private(&path, wanted.as_bytes()).ok()?;
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
    let catalogue = devpit_agentcli::read_every_agent(&devpit_agentcli::seed_sources());
    Ok(devpit_rpc::Agents {
        agents: catalogue
            .agents
            .into_iter()
            .map(|agent| devpit_rpc::Agent {
                name: agent.name,
                description: agent.description,
                model: agent.model,
                source: agent.source,
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
