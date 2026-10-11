//! `mcp.health` — how an account's MCP servers answer, from where the project
//! stands (a project can declare its own). Kept per account and folder, so a
//! session's start reads the last answer instead of waiting on a new one.

use std::path::Path;
use std::sync::OnceLock;

use devpit_rpc::{ErrorCode, McpHealth, RpcError};

use crate::mcp_cache::{Kept, Known, FRESH_FOR};

fn kept() -> &'static Kept<McpHealth> {
    static KEPT: OnceLock<Kept<McpHealth>> = OnceLock::new();
    KEPT.get_or_init(Kept::default)
}

/// No default chosen is the bare CLI, which is also what `claude` starts.
fn key(profile_id: &str, cwd: &Path) -> String {
    let profile = if profile_id.is_empty() {
        "claude"
    } else {
        profile_id
    };
    format!("{profile}\n{}", cwd.display())
}

/// Asks the CLI, as `profile_id`'s account and from `cwd`; seconds, not less.
fn asked(profile_id: &str, cwd: &Path) -> Result<McpHealth, RpcError> {
    let store = crate::projects::store()?;
    // A shell function has nothing to spawn: the plain CLI answers for it.
    let runner = crate::agent_profiles::runner_for(&store, profile_id).ok();
    let servers = devpit_agentcli::mcp_health(runner.as_ref(), cwd)
        .map_err(|err| RpcError::new(ErrorCode::Conflict, err.to_string()))?;
    Ok(McpHealth {
        profile_id: profile_id.to_owned(),
        servers,
    })
}

/// What was last heard for `profile_id` in `cwd`, without waiting; asked
/// again behind the caller when it is old or was never asked.
pub(crate) fn kept_or_asked(profile_id: &str, cwd: &Path) -> Option<McpHealth> {
    let (profile, at) = (profile_id.to_owned(), cwd.to_path_buf());
    kept()
        .read_and_refresh(&key(profile_id, cwd), FRESH_FOR, move || {
            asked(&profile, &at).ok()
        })
        .value()
}

/// The default account's servers. `again` asks the CLI even when the last
/// answer is recent: the footer's Refresh.
#[tauri::command]
#[specta::specta]
pub async fn mcp_health(project_id: Option<String>, again: bool) -> Result<McpHealth, RpcError> {
    crate::off_main::blocking(move || {
        let profile_id = crate::agent_choice::default_id(&crate::projects::store()?);
        let cwd = match project_id.as_deref() {
            Some(id) => crate::roots::root_of(id, None)?,
            None => crate::installations::home()?,
        };
        let key = key(&profile_id, &cwd);
        if let (false, Known::Fresh(health)) = (again, kept().known(&key, FRESH_FOR)) {
            return Ok(health);
        }
        let health = asked(&profile_id, &cwd)?;
        kept().put(&key, health.clone());
        Ok(health)
    })
    .await
}
