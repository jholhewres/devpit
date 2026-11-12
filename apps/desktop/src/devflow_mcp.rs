//! The standalone devflow added to other tools as an MCP server, `devflow
//! mcp`, in each tool's own configuration: Codex's `config.toml`, Cursor's
//! and Gemini's `mcpServers`, OpenCode's `mcp`. Each write is the person's
//! click, and everything else in the file is left as it was.

use std::path::{Path, PathBuf};

use devpit_rpc::{DevflowMcp, DevflowMcpTool, ErrorCode, RpcError};
use serde_json::{json, Value};

/// The name devflow is added under in every tool.
const NAME: &str = "devflow";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tool {
    Codex,
    Cursor,
    Opencode,
    Gemini,
}

pub(crate) const TOOLS: [Tool; 4] = [Tool::Codex, Tool::Cursor, Tool::Opencode, Tool::Gemini];

impl Tool {
    pub(crate) fn id(self) -> &'static str {
        match self {
            Tool::Codex => "codex",
            Tool::Cursor => "cursor",
            Tool::Opencode => "opencode",
            Tool::Gemini => "gemini",
        }
    }

    pub(crate) fn label(self) -> &'static str {
        match self {
            Tool::Codex => "Codex",
            Tool::Cursor => "Cursor",
            Tool::Opencode => "OpenCode",
            Tool::Gemini => "Gemini CLI",
        }
    }

    pub(crate) fn of(id: &str) -> Option<Tool> {
        TOOLS.into_iter().find(|tool| tool.id() == id)
    }

    /// Its configuration file, from the home folder and the environment.
    pub(crate) fn file(self, home: &Path, env: impl Fn(&str) -> Option<String>) -> PathBuf {
        let set = |key: &str| env(key).filter(|one| !one.is_empty()).map(PathBuf::from);
        match self {
            Tool::Codex => set("CODEX_HOME")
                .unwrap_or_else(|| home.join(".codex"))
                .join("config.toml"),
            Tool::Cursor => home.join(".cursor").join("mcp.json"),
            Tool::Opencode => set("XDG_CONFIG_HOME")
                .unwrap_or_else(|| home.join(".config"))
                .join("opencode")
                .join("opencode.json"),
            Tool::Gemini => home.join(".gemini").join("settings.json"),
        }
    }

    /// The JSON key servers sit under, for the tools whose file is JSON.
    fn servers(self) -> &'static str {
        match self {
            Tool::Opencode => "mcp",
            _ => "mcpServers",
        }
    }

    fn entry(self, command: &str) -> Value {
        match self {
            Tool::Opencode => {
                json!({ "type": "local", "command": [command, "mcp"], "enabled": true })
            }
            _ => json!({ "command": command, "args": ["mcp"] }),
        }
    }
}

/// Codex's `config.toml`, read as TOML: an entry however it is spelled.
fn codex_doc(text: &str) -> Result<toml_edit::DocumentMut, String> {
    text.parse().map_err(|_| {
        "Codex's config.toml is not valid TOML devpit can change; it is left as it was".to_owned()
    })
}

/// Line ends as the file had them: toml_edit writes what it adds with `\n`.
fn codex_text(doc: &toml_edit::DocumentMut, was: &str) -> String {
    let text = doc.to_string();
    if was.contains("\r\n") {
        text.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        text
    }
}

/// Whether `text`, the tool's file, has devflow in it.
pub(crate) fn added_in(tool: Tool, text: &str) -> bool {
    match tool {
        Tool::Codex => codex_doc(text).ok().is_some_and(|doc| {
            doc.get("mcp_servers")
                .and_then(|all| all.get(NAME))
                .is_some()
        }),
        _ => serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}'))
            .ok()
            .is_some_and(|value| value[tool.servers()].get(NAME).is_some()),
    }
}

/// `text` (none: no file yet) with devflow added, run as `command mcp`.
pub(crate) fn with(tool: Tool, text: Option<&str>, command: &str) -> Result<String, String> {
    if tool == Tool::Codex {
        let text = text.unwrap_or_default();
        let mut doc = codex_doc(text)?;
        let beside_others = !text.trim().is_empty();
        let servers = doc
            .entry("mcp_servers")
            .or_insert_with(|| {
                let mut all = toml_edit::Table::new();
                all.set_implicit(true);
                toml_edit::Item::Table(all)
            })
            .as_table_like_mut()
            .ok_or("Codex's `mcp_servers` is not a table; it is left as it was")?;
        let args = toml_edit::value(toml_edit::Array::from_iter(["mcp"]));
        match servers
            .get_mut(NAME)
            .and_then(toml_edit::Item::as_table_like_mut)
        {
            Some(ours) => {
                ours.insert("command", toml_edit::value(command));
                ours.insert("args", args);
            }
            None => {
                let mut ours = toml_edit::Table::new();
                if beside_others {
                    ours.decor_mut().set_prefix("\n");
                }
                ours.insert("command", toml_edit::value(command));
                ours.insert("args", args);
                servers.insert(NAME, toml_edit::Item::Table(ours));
            }
        }
        return Ok(codex_text(&doc, text));
    }
    match text.filter(|one| !one.trim().is_empty()) {
        Some(one) => {
            devpit_agentcli::json_in_place::put(one, tool.servers(), NAME, &tool.entry(command))
                .map_err(|_| not_ours(tool))
        }
        None => pretty(&json!({ tool.servers(): { NAME: tool.entry(command) } })),
    }
}

/// `text` without devflow.
pub(crate) fn without(tool: Tool, text: &str) -> Result<String, String> {
    if tool == Tool::Codex {
        let mut doc = codex_doc(text)?;
        if let Some(servers) = doc
            .get_mut("mcp_servers")
            .and_then(toml_edit::Item::as_table_like_mut)
        {
            servers.remove(NAME);
        }
        return Ok(codex_text(&doc, text));
    }
    devpit_agentcli::json_in_place::take(text, tool.servers(), NAME).map_err(|_| not_ours(tool))
}

fn not_ours(tool: Tool) -> String {
    format!(
        "{}'s file is not a JSON object devpit can rewrite; it is left as it was",
        tool.label()
    )
}

fn pretty(value: &Value) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|text| text + "\n")
        .map_err(|err| err.to_string())
}

/// Every tool's file under `home`, and whether devflow is in it.
pub(crate) fn state_in(
    home: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    command: &str,
) -> DevflowMcp {
    let tools = TOOLS
        .into_iter()
        .map(|tool| {
            let file = tool.file(home, env);
            let added = std::fs::read_to_string(&file).is_ok_and(|text| added_in(tool, &text));
            DevflowMcpTool {
                tool: tool.id().to_owned(),
                label: tool.label().to_owned(),
                file: file.display().to_string(),
                added,
            }
        })
        .collect();
    DevflowMcp {
        command: command.to_owned(),
        tools,
    }
}

/// devflow added to `tool`'s file under `home`, or taken out of it.
pub(crate) fn set_in(
    home: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    tool: Tool,
    added: bool,
    command: &str,
) -> Result<(), String> {
    let file = tool.file(home, env);
    let before = match std::fs::read_to_string(&file) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("{} could not be read: {err}", file.display())),
    };
    let after = match (added, before.as_deref()) {
        (true, text) => with(tool, text, command)?,
        (false, Some(text)) if added_in(tool, text) => without(tool, text)?,
        (false, _) => return Ok(()),
    };
    devpit_agentcli::config_edit::write_text(&file, before.as_deref(), &after).map(|_| ())
}

fn home() -> Result<PathBuf, RpcError> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| RpcError::internal("HOME is not set"))
}

fn env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// The installed standalone devflow, else the one on the tool's PATH.
fn command() -> String {
    crate::devflow_binary::bin_dir()
        .map(|dir| dir.join(NAME))
        .ok()
        .filter(|bin| bin.is_file())
        .map_or_else(|| NAME.to_owned(), |bin| bin.display().to_string())
}

/// `devflow.mcp` — devflow as an MCP server in each tool, as their files say.
#[tauri::command]
#[specta::specta]
pub async fn devflow_mcp() -> Result<DevflowMcp, RpcError> {
    crate::off_main::blocking(|| Ok(state_in(&home()?, &env, &command()))).await
}

/// `devflow.mcp_set` — the person's click: devflow added to `tool`, or removed.
#[tauri::command]
#[specta::specta]
pub async fn devflow_mcp_set(tool: String, added: bool) -> Result<DevflowMcp, RpcError> {
    crate::off_main::blocking(move || {
        let which = Tool::of(&tool).ok_or_else(|| {
            RpcError::new(
                ErrorCode::Invalid,
                format!("{tool} is not a tool devflow is added to"),
            )
        })?;
        let (home, command) = (home()?, command());
        set_in(&home, &env, which, added, &command).map_err(RpcError::internal)?;
        Ok(state_in(&home, &env, &command))
    })
    .await
}

#[cfg(test)]
#[path = "devflow_mcp_tests.rs"]
mod tests;
