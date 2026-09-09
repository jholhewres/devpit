//! The MCP servers the agent CLI is configured with — read, never written.
//!
//! The decision on record: devpit does not manage MCP. The CLI already owns
//! that catalogue, and two sources of truth diverge on the first
//! `claude mcp add`. This panel exists because a server that failed to
//! connect looks exactly like a tool the agent never had.

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub name: String,
    /// `project` or `user` — which file names it.
    pub scope: String,
    /// The command or URL it is reached by, for the row's second line.
    pub reached_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Servers {
    pub servers: Vec<Server>,
    /// The files this was read from, so the panel can say whose catalogue it
    /// is showing.
    pub sources: Vec<String>,
    /// Why nothing could be read. A panel that says why beats a panel that
    /// looks empty.
    pub problem: Option<String>,
    /// The command that manages them, to copy. Never run from here.
    pub manage_with: String,
}

/// The servers named in one config file.
///
/// Both shapes are accepted: `{"mcpServers": {...}}` as the file's whole
/// content, which is `.mcp.json`, and the same key nested under a project,
/// which is how `~/.claude.json` holds it.
pub fn servers_in(text: &str, scope: &str) -> Vec<Server> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    let Some(map) = value.get("mcpServers").and_then(|found| found.as_object()) else {
        return Vec::new();
    };
    let mut servers: Vec<Server> = map
        .iter()
        .map(|(name, config)| Server {
            name: name.clone(),
            scope: scope.to_owned(),
            reached_by: reached_by(config),
        })
        .collect();
    servers.sort_by(|a, b| a.name.cmp(&b.name));
    servers
}

/// How a server is reached, in one line.
fn reached_by(config: &serde_json::Value) -> String {
    if let Some(url) = config.get("url").and_then(|found| found.as_str()) {
        return url.to_owned();
    }
    let command = config
        .get("command")
        .and_then(|found| found.as_str())
        .unwrap_or_default();
    let args: Vec<&str> = config
        .get("args")
        .and_then(|found| found.as_array())
        .map(|list| list.iter().filter_map(|one| one.as_str()).collect())
        .unwrap_or_default();
    if args.is_empty() {
        command.to_owned()
    } else {
        format!("{command} {}", args.join(" "))
    }
}

fn read(path: &Path, scope: &str, into: &mut Vec<Server>, sources: &mut Vec<String>) {
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    let found = servers_in(&text, scope);
    if found.is_empty() {
        return;
    }
    sources.push(path.display().to_string());
    // The project's own file wins a name it shares with the user's: that is
    // the order the CLI resolves them in.
    for server in found {
        if !into.iter().any(|had: &Server| had.name == server.name) {
            into.push(server);
        }
    }
}

/// `mcp.list` — what the CLI resolved for this project.
#[tauri::command]
#[specta::specta]
pub fn mcp_list(project_id: Option<String>) -> Result<Servers, RpcError> {
    let mut servers = Vec::new();
    let mut sources = Vec::new();

    if let Some(id) = project_id {
        let store = Store::open_default()?;
        if let Some(row) = store.project(&id)? {
            read(
                &PathBuf::from(&row.root_path).join(".mcp.json"),
                "project",
                &mut servers,
                &mut sources,
            );
        }
    }
    if let Some(home) = std::env::var_os("HOME").map(PathBuf::from) {
        read(
            &home.join(".claude.json"),
            "user",
            &mut servers,
            &mut sources,
        );
        read(&home.join(".mcp.json"), "user", &mut servers, &mut sources);
    } else {
        return Err(RpcError::new(
            ErrorCode::Internal,
            "there is no HOME to read the CLI's config from",
        ));
    }

    Ok(Servers {
        problem: servers
            .is_empty()
            .then(|| "no MCP server is configured for the agent CLI".to_owned()),
        manage_with: "claude mcp".to_owned(),
        servers,
        sources,
    })
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
