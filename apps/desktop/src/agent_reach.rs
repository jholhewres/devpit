//! How an agent devpit starts finds devpit: the MCP server on its command
//! line, and `devpit-agent` on its PATH.
//!
//! On the command line and nowhere else, for the reason the hooks are
//! (`shell_launch::launch_line`): a flag reaches the agents this app started,
//! and editing a tool's own configuration would reach every agent the person
//! ever starts, which is not a menu item's to decide.
//!
//! The server is this binary (`devpit mcp`, `devpit-agentapi`). From an
//! AppImage that is the AppImage file, not the mount it runs from, which moves
//! on every start.

use std::path::{Path, PathBuf};

/// The binary an agent runs to reach devpit.
pub(crate) fn exe() -> Option<PathBuf> {
    match std::env::var_os("APPIMAGE") {
        Some(appimage) if !appimage.is_empty() => Some(PathBuf::from(appimage)),
        _ => std::env::current_exe().ok(),
    }
}

/// What to add to an agent's launch line so it has devpit's tools, or
/// nothing for an agent with no per-session way to be given them.
///
/// `config` is where Claude's MCP file goes; it is written here, and only
/// when it differs.
pub(crate) fn mcp_flags(agent: &str, exe: &Path, config: &Path) -> Option<String> {
    let exe = exe.to_str()?;
    // Quoted into a shell line and, for Codex, into TOML inside it: a path
    // with either quote in it is not one this can carry safely.
    if exe.contains(['\'', '"', '\\']) {
        return None;
    }
    match agent {
        "claude" => {
            let wanted = claude_config(exe, agent);
            if std::fs::read_to_string(config).ok().as_deref() != Some(wanted.as_str()) {
                devpit_core::home::write_private(config, wanted.as_bytes()).ok()?;
            }
            let path = config.to_str()?;
            (!path.contains('\'')).then(|| format!("--mcp-config '{path}'"))
        }
        "codex" => Some(format!(
            "-c 'mcp_servers.devpit.command=\"{exe}\"' -c 'mcp_servers.devpit.args=[\"mcp\"]' \
             -c 'mcp_servers.devpit.env={{DEVPIT_AGENT_ID=\"codex\"}}'"
        )),
        _ => None,
    }
}

/// Claude's `--mcp-config` file: devpit as a stdio server, told which agent
/// it serves so a comment is signed with it.
pub(crate) fn claude_config(exe: &str, agent: &str) -> String {
    serde_json::json!({
        "mcpServers": {
            "devpit": {
                "type": "stdio",
                "command": exe,
                "args": ["mcp"],
                "env": { "DEVPIT_AGENT_ID": agent },
            }
        }
    })
    .to_string()
}

/// `<root>/bin/devpit-agent`, a two-line script that runs `devpit agent`.
///
/// Not `devpit`: that name is the installed app itself, on the person's own
/// PATH, and a terminal where `devpit` meant something else would be a trap.
pub(crate) fn cli_shim(root: &Path, exe: &Path) -> Option<PathBuf> {
    let exe = exe.to_str()?;
    if exe.contains('\'') {
        return None;
    }
    let dir = root.join("bin");
    let path = dir.join("devpit-agent");
    let wanted = format!("#!/bin/sh\nexec '{exe}' agent \"$@\"\n");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(wanted.as_str()) {
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::write(&path, &wanted).ok()?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).ok()?;
        }
    }
    Some(dir)
}

#[cfg(test)]
#[path = "agent_reach_tests.rs"]
mod tests;
