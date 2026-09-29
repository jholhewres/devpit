//! `mcp.health` — how the default account's MCP servers answer, from where
//! the project stands (a project can declare its own).

use devpit_rpc::{ErrorCode, McpHealth, RpcError};

#[tauri::command]
#[specta::specta]
pub async fn mcp_health(project_id: Option<String>) -> Result<McpHealth, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let profile_id = crate::agent_choice::default_id(&store);
        // A shell function has nothing to spawn: the plain CLI answers for it.
        let runner = crate::agent_profiles::runner_for(&store, &profile_id).ok();
        let cwd = match project_id.as_deref() {
            Some(id) => crate::roots::root_of(id, None)?,
            None => crate::installations::home()?,
        };
        let servers = devpit_agentcli::mcp_health(runner.as_ref(), &cwd)
            .map_err(|err| RpcError::new(ErrorCode::Conflict, err.to_string()))?;
        Ok(McpHealth {
            profile_id,
            servers,
        })
    })
    .await
}
