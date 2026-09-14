//! The two shapes an MCP server is written in, and what to call it.
//!
//! Apart from `mcp.rs` because that file is the command — which files to read,
//! in which order, and what to say when there is nothing. This one is the
//! grammar, which is the part with cases in it.

use crate::mcp::Server;

/// The servers named at the top level of a config file.
///
/// `{"mcpServers": {...}}` as the file's whole content, which is `.mcp.json`
/// and the user-wide section of `~/.claude.json`.
pub(crate) fn servers_in(text: &str, scope: &str) -> Vec<Server> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    named(value.get("mcpServers"), scope)
}

/// The servers the CLI keeps for one working directory.
///
/// `~/.claude.json` holds these under `projects.<absolute path>.mcpServers`,
/// keyed by the directory the CLI was started in. Nothing read them before,
/// while the comment above `servers_in` claimed they were — measured on one
/// machine, five servers across five projects, none of them ever shown.
pub(crate) fn servers_for(text: &str, root: &str, scope: &str) -> Vec<Server> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    named(
        value
            .get("projects")
            .and_then(|projects| projects.get(root))
            .and_then(|project| project.get("mcpServers")),
        scope,
    )
}

/// One `mcpServers` object as rows, sorted so the panel does not reshuffle.
fn named(found: Option<&serde_json::Value>, scope: &str) -> Vec<Server> {
    let Some(map) = found.and_then(|found| found.as_object()) else {
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

#[cfg(test)]
#[path = "mcp_reading_tests.rs"]
mod tests;
