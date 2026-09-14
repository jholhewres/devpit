//! The MCP servers the agent CLI is configured with — read, never written.
//!
//! The decision on record: devpit does not manage MCP. The CLI already owns
//! that catalogue, and two sources of truth diverge on the first
//! `claude mcp add`. This panel exists because a server that failed to
//! connect looks exactly like a tool the agent never had.
//!
//! Which makes reading the *right* installation the whole job: the CLI's
//! configuration is not always in `~/.claude` — see `cli_config`.

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::RpcError;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::mcp_reading::{servers_for, servers_in};

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
    /// The CLI configuration directory these came from.
    pub directory: String,
}

/// Takes the servers a file names, keeping the first spelling of each name.
///
/// The project's own file wins a name it shares with the user's: that is the
/// order the CLI resolves them in, so it is the order they are read in here.
fn take(found: Vec<Server>, path: &Path, into: &mut Vec<Server>, sources: &mut Vec<String>) {
    if found.is_empty() {
        return;
    }
    sources.push(path.display().to_string());
    for server in found {
        if !into.iter().any(|had: &Server| had.name == server.name) {
            into.push(server);
        }
    }
}

/// `mcp.list` — what one installation of the CLI resolved for this project,
/// the default profile's when none is named.
#[tauri::command]
#[specta::specta]
pub fn mcp_list(
    project_id: Option<String>,
    directory: Option<String>,
) -> Result<Servers, RpcError> {
    let home = crate::installations::home()?;
    let chosen = crate::installations::chosen(directory.as_deref())?;
    let cli = chosen.directory;
    let settings = devpit_agentcli::cli_config::settings_file_from(&home, chosen.said.as_deref());
    // Read once: it holds every project the CLI was ever started in, and is
    // consulted twice below.
    let settings_text = text_of(&settings);

    let mut servers = Vec::new();
    let mut sources = Vec::new();

    if let Some(id) = project_id {
        let store = Store::open_default()?;
        if let Some(row) = store.project(&id)? {
            let root = PathBuf::from(&row.root_path);
            let checked_in = root.join(".mcp.json");
            take(
                servers_in(&text_of(&checked_in), "project"),
                &checked_in,
                &mut servers,
                &mut sources,
            );
            // The CLI also keeps per-directory servers in its own settings,
            // keyed by the path it was started in.
            take(
                servers_for(&settings_text, &row.root_path, "project"),
                &settings,
                &mut servers,
                &mut sources,
            );
        }
    }

    take(
        servers_in(&settings_text, "user"),
        &settings,
        &mut servers,
        &mut sources,
    );
    let mine = home.join(".mcp.json");
    take(
        servers_in(&text_of(&mine), "user"),
        &mine,
        &mut servers,
        &mut sources,
    );
    sources.dedup();

    Ok(Servers {
        problem: servers
            .is_empty()
            .then(|| "no MCP server is configured for the agent CLI".to_owned()),
        manage_with: "claude mcp".to_owned(),
        directory: cli.display().to_string(),
        servers,
        sources,
    })
}

/// A file's text, or nothing at all — an absent config is the ordinary case.
fn text_of(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[cfg(test)]
#[path = "mcp_tests.rs"]
mod tests;
