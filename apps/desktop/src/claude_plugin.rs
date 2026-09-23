//! The devpit plugin for Claude Code: the hooks and the board's tools, for
//! every session of an installation, however it was started.
//!
//! The flags on a launch line (`shell_launch::launch_line`) reach only the
//! agents devpit starts, and the shell wrappers step aside for a person's own
//! `claude` function — so `claudin`, `glm` and the like ran with neither the
//! hooks nor the tools. A plugin is how Warp reaches the same sessions, and
//! it is the person's choice: nothing here runs until they press Install.
//!
//! The plugin is a folder devpit writes and adds as a local marketplace. Its
//! hooks run only inside a devpit terminal, and both halves stand aside where
//! the flags already brought them (`plugin_hooks_json`, `mcp::FROM_PLUGIN`).

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::RpcError;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::installations::Found;

/// The marketplace and the plugin share a name: `devpit@devpit`. A
/// development build installs its own, beside the released one rather than
/// over it: the two point at different binaries and different homes.
const NAME: &str = if cfg!(debug_assertions) {
    "devpit-dev"
} else {
    "devpit"
};
const KEY: &str = if cfg!(debug_assertions) {
    "devpit-dev@devpit-dev"
} else {
    "devpit@devpit"
};

/// Where the plugin stands in one installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ClaudePluginState {
    Missing,
    /// Installed from an earlier build of the plugin.
    Outdated,
    Current,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePluginInstallation {
    pub directory: String,
    pub profiles: Vec<String>,
    pub state: ClaudePluginState,
}

/// The folder devpit adds as a marketplace.
fn folder(root: &Path) -> PathBuf {
    root.join("claude-plugin")
}

/// The plugin's files, by path under [`folder`], at a version that changes
/// whenever any of them does — which is what tells an install it is behind.
pub(crate) fn bundle(root: &Path, exe: &str) -> (String, Vec<(PathBuf, String)>) {
    let hooks = devpit_agentcli::plugin_hooks_json(
        &devpit_agentcli::endpoint_file(root),
        &devpit_agentcli::auth_file(root),
    );
    let mcp = serde_json::json!({
        "mcpServers": {
            "devpit": {
                "command": exe,
                "args": ["mcp"],
                "env": { "DEVPIT_AGENT_ID": "claude", (devpit_agentapi::mcp::FROM_PLUGIN): "1" },
            }
        }
    })
    .to_string();
    let version = format!("1.0.0-b{:08x}", fnv(&[hooks.as_bytes(), mcp.as_bytes()]));
    let about = "devpit's hooks and board tools, inside devpit terminals";
    let plugin = serde_json::json!({
        "name": NAME, "version": version, "description": about,
        "author": { "name": "devpit" },
    })
    .to_string();
    let market = serde_json::json!({
        "name": NAME,
        "owner": { "name": "devpit" },
        "metadata": { "description": about },
        "plugins": [{ "name": NAME, "source": "./plugin", "version": version, "description": about }],
    })
    .to_string();
    let files = vec![
        (PathBuf::from(".claude-plugin/marketplace.json"), market),
        (PathBuf::from("plugin/.claude-plugin/plugin.json"), plugin),
        (PathBuf::from("plugin/hooks/hooks.json"), hooks),
        (PathBuf::from("plugin/.mcp.json"), mcp),
    ];
    (version, files)
}

/// FNV-1a: stable across builds, which `DefaultHasher` does not promise.
fn fnv(parts: &[&[u8]]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5;
    for byte in parts.iter().flat_map(|part| part.iter()) {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Writes the plugin where the marketplace points, only what differs.
fn write(root: &Path) -> Result<(PathBuf, String), RpcError> {
    let exe = crate::agent_reach::exe()
        .and_then(|exe| exe.to_str().map(str::to_owned))
        .ok_or_else(|| RpcError::internal("devpit cannot tell where its own binary is"))?;
    let (version, files) = bundle(root, &exe);
    let base = folder(root);
    for (path, text) in files {
        let path = base.join(path);
        if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| RpcError::internal(err.to_string()))?;
        }
        std::fs::write(&path, text).map_err(|err| RpcError::internal(err.to_string()))?;
    }
    Ok((base, version))
}

/// What `installed_plugins.json` in `directory` says of the plugin.
pub(crate) fn state_in(directory: &Path, version: &str) -> ClaudePluginState {
    let file = directory.join("plugins").join("installed_plugins.json");
    let Ok(text) = std::fs::read_to_string(file) else {
        return ClaudePluginState::Missing;
    };
    let Ok(read) = serde_json::from_str::<serde_json::Value>(&text) else {
        return ClaudePluginState::Missing;
    };
    let Some(installed) = read["plugins"][KEY]
        .as_array()
        .filter(|all| !all.is_empty())
    else {
        return ClaudePluginState::Missing;
    };
    if installed.iter().any(|one| one["version"] == version) {
        ClaudePluginState::Current
    } else {
        ClaudePluginState::Outdated
    }
}

fn version_now() -> Result<(PathBuf, String), RpcError> {
    let root = Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    let exe = crate::agent_reach::exe()
        .and_then(|exe| exe.to_str().map(str::to_owned))
        .unwrap_or_default();
    Ok((root.clone(), bundle(&root, &exe).0))
}

/// `claude_plugin.state` — the plugin in each Claude installation.
#[tauri::command]
#[specta::specta]
pub async fn claude_plugin_state() -> Result<Vec<ClaudePluginInstallation>, RpcError> {
    crate::off_main::blocking(claude_plugin_state_now).await
}

pub(crate) fn claude_plugin_state_now() -> Result<Vec<ClaudePluginInstallation>, RpcError> {
    let (_, version) = version_now()?;
    Ok(crate::installations::found()?
        .into_iter()
        .map(|one| ClaudePluginInstallation {
            state: state_in(&one.directory, &version),
            directory: one.directory.display().to_string(),
            profiles: one.profiles,
        })
        .collect())
}

/// `claude_plugin.install` — installs or brings up to date the plugin in every
/// Claude installation that is behind. Answers what each one ended as.
#[tauri::command]
#[specta::specta]
pub async fn claude_plugin_install() -> Result<Vec<ClaudePluginInstallation>, RpcError> {
    crate::off_main::blocking(claude_plugin_install_now).await
}

pub(crate) fn claude_plugin_install_now() -> Result<Vec<ClaudePluginInstallation>, RpcError> {
    let (root, _) = version_now()?;
    let (base, version) = write(&root)?;
    let base = base.display().to_string();
    for one in crate::installations::found()? {
        match state_in(&one.directory, &version) {
            ClaudePluginState::Current => continue,
            ClaudePluginState::Outdated => {
                // A local marketplace is read when it is added or updated,
                // and an install copies from it: uninstalling first is what
                // makes the copy the one just written.
                let _ = claude(&one, &["plugin", "marketplace", "update", NAME]);
                let _ = claude(&one, &["plugin", "uninstall", KEY]);
            }
            ClaudePluginState::Missing => {
                // Already there after an uninstall, which is fine: an add
                // that fails for that reason leaves the right marketplace.
                let _ = claude(&one, &["plugin", "marketplace", "add", &base]);
                let _ = claude(&one, &["plugin", "marketplace", "update", NAME]);
            }
        }
        claude(&one, &["plugin", "install", KEY])?;
    }
    claude_plugin_state_now()
}

/// Runs `claude` against one installation, and says what went wrong in its
/// own words.
fn claude(installation: &Found, args: &[&str]) -> Result<(), RpcError> {
    let mut command = devpit_pty::host_env::command("claude");
    command.args(args).stdin(std::process::Stdio::null());
    // The directory as resolved, not as the profile spelled it: `~` in a
    // variable is not expanded by anybody on the way to the CLI.
    match &installation.said {
        Some(_) => command.env("CLAUDE_CONFIG_DIR", &installation.directory),
        None => command.env_remove("CLAUDE_CONFIG_DIR"),
    };
    let out = command
        .output()
        .map_err(|err| RpcError::internal(format!("claude could not be run: {err}")))?;
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(&out.stderr);
    let said = if said.trim().is_empty() {
        String::from_utf8_lossy(&out.stdout).into_owned()
    } else {
        said.into_owned()
    };
    Err(RpcError::internal(format!(
        "`claude {}` failed: {}",
        args.join(" "),
        said.trim()
    )))
}

#[cfg(test)]
#[path = "claude_plugin_tests.rs"]
mod tests;
