//! Agents mode: an orchestrator that is the person's right hand. It plans,
//! hands the work to agents of its own and follows it up. Off by default and
//! per orchestrator, kept in its settings, out of its own chat's reach.
//!
//! On, devpit writes `.devpit/agents.md`, its own like `.devpit/orchestrator.md`,
//! imports it into the orchestrator's `CLAUDE.md` under devpit's briefs, and
//! lists the agents' MCP tools. Off, the import and the file go, and the
//! orchestrator's files and tool list are the ones they always were.

use std::path::Path;

use devpit_core::Store;
use devpit_rpc::{AgentsMode, Project, RpcError};
use serde_json::{json, Value};
use tauri::AppHandle;

use crate::session_locale::SessionLocale;

/// devpit's agents brief, beside its orchestrator brief.
const BRIEF: &str = ".devpit/agents.md";
/// The line that imports it.
pub(crate) const IMPORT: &str = "@.devpit/agents.md";
/// Its key among the orchestrator's settings.
const KEY: &str = "agents";
/// Why a tool of the mode is refused.
const OFF: &str = "Agents mode is off for this orchestrator: the person turns it on in its chat";

/// The brief's words in the session's language: the pt-BR ones beside the
/// English base, which every other language reads too.
fn words(locale: SessionLocale) -> &'static str {
    match locale.tag() {
        "pt-BR" => include_str!("agents_mode_devpit_pt_br.md"),
        _ => include_str!("agents_mode_devpit.md"),
    }
}

/// Whether the orchestrator in `folder` has Agents mode on.
pub(crate) fn on(store: &Store, folder: &Path) -> bool {
    set(store, folder) == Some(true)
}

/// The mode as the person last set it, or nothing when they never did.
pub(crate) fn set(store: &Store, folder: &Path) -> Option<bool> {
    store
        .orchestrator_settings(folder)
        .ok()
        .and_then(|all| all[KEY].as_bool())
}

/// The brief and its import as the mode says: written and imported when on,
/// taken out when off, in the language of the session the folder is being
/// written for.
pub(crate) fn brief_in(folder: &Path, on: bool, locale: SessionLocale) -> std::io::Result<()> {
    crate::orchestrator_brief::mode_brief(folder, BRIEF, words(locale), IMPORT, on)
}

/// The methods only an orchestrator in Agents mode is answered, for the agent
/// API's list.
pub(crate) const METHODS: [&str; 1] = ["agents"];

/// One of [`METHODS`], asked by the orchestrator `here`.
pub(crate) fn respond(method: &str, here: &Project) -> Result<Value, String> {
    if here.orchestrator.is_none() {
        return Err(OFF.to_owned());
    }
    let store = crate::projects::store().map_err(|err| err.message)?;
    answer(&store, method, here)
}

/// [`respond`], with the mode read from `store`.
fn answer(store: &Store, method: &str, here: &Project) -> Result<Value, String> {
    if here.orchestrator.is_none() || !on(store, Path::new(&here.root_path)) {
        return Err(OFF.to_owned());
    }
    match method {
        // Nothing makes an agent yet, so the roster is the empty one.
        "agents" => Ok(json!({ "agents": [] })),
        _ => Err(format!("devpit does not answer `{method}`")),
    }
}

/// `agents.mode` — whether this orchestrator's Agents mode is on.
#[tauri::command]
#[specta::specta]
pub async fn agents_mode(project_id: String) -> Result<AgentsMode, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let folder = crate::assistant::folder_of(&store, &project_id)?;
        Ok(AgentsMode {
            on: on(&store, &folder),
        })
    })
    .await
}

/// `agents.mode_set` — Agents mode on or off, as the person clicked it. The
/// MCP servers are told to look again, so the running chat gains or loses the
/// agents' tools; its brief is read when the chat next starts.
#[tauri::command]
#[specta::specta]
pub async fn agents_mode_set(
    app: AppHandle,
    project_id: String,
    on: bool,
) -> Result<AgentsMode, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let folder = crate::assistant::folder_of(&store, &project_id)?;
        // The brief first: a write refused leaves the mode as it was.
        brief_in(&folder, on, SessionLocale::capture())
            .map_err(|err| RpcError::internal(err.to_string()))?;
        store.set_orchestrator_setting(&folder, KEY, json!(on))?;
        crate::devflow_asks::rang(Some(&app), &project_id);
        Ok(AgentsMode { on })
    })
    .await
}

#[cfg(test)]
#[path = "agents_mode_tests.rs"]
mod tests;
