//! Which agent is the default, and which ones are offered at all.
//!
//! Two facts about the catalogue rather than about any one profile, which is
//! why they are here and not in `agent_profiles`: a person with twelve agents
//! on the list uses two of them, and a menu that offers all twelve every time
//! is a menu they read past.
//!
//! Both are preferences and neither is a migration: a default that points at
//! something no longer installed is read as no default at all, and a disabled
//! id that nothing answers to is simply never consulted.

use devpit_core::store::preference;
use devpit_rpc::RpcError;
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentChoice {
    /// What a new terminal opens, by agent or profile id. Empty for none,
    /// which is a plain shell — a deliberate answer, not a missing one.
    pub default_id: String,
    /// Ids kept out of the menus. Off the catalogue, not uninstalled.
    pub disabled: Vec<String>,
    /// Whether an agent devpit starts is told to report what it is doing.
    ///
    /// The hooks travel on the command line and reach only the agents this app
    /// starts — nothing is written into anybody's own configuration, so
    /// turning this off is the whole of turning it off.
    pub hooks: bool,
}

fn read(store: &devpit_core::Store) -> AgentChoice {
    let default_id = store
        .preference(preference::AGENT_DEFAULT)
        .ok()
        .flatten()
        .unwrap_or_default();
    let disabled = store
        .preference(preference::AGENTS_DISABLED)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    AgentChoice {
        default_id,
        disabled,
        // Unset is on: the sidebar saying whether an agent is waiting for you
        // is what the flag is for, and a fresh install should have it.
        hooks: store
            .preference_flag(preference::AGENT_HOOKS)
            .ok()
            .flatten()
            .unwrap_or(true),
    }
}

/// Whether an agent devpit starts is told to report what it is doing.
pub(crate) fn hooks_on(store: &devpit_core::Store) -> bool {
    read(store).hooks
}

/// What a new terminal opens, for whoever needs to start where it starts.
pub(crate) fn default_id(store: &devpit_core::Store) -> String {
    read(store).default_id
}

/// The ids kept out of the menus, for whoever is drawing one.
pub(crate) fn disabled(store: &devpit_core::Store) -> Vec<String> {
    read(store).disabled
}

/// `agent.choice` — the default and the ones switched off.
#[tauri::command]
#[specta::specta]
pub fn agent_choice() -> Result<AgentChoice, RpcError> {
    Ok(read(&crate::projects::store()?))
}

/// `agent.default_set` — what a new terminal opens.
#[tauri::command]
#[specta::specta]
pub fn agent_default_set(id: String) -> Result<AgentChoice, RpcError> {
    let store = crate::projects::store()?;
    store.set_preference(preference::AGENT_DEFAULT, &id)?;
    Ok(read(&store))
}

/// `agent.enabled_set` — whether this one is offered.
///
/// The default cannot be switched off: a menu whose default is not in it is a
/// menu that opens nothing and explains nothing. Switching off the default
/// clears it instead, which is a state the screen can draw.
#[tauri::command]
#[specta::specta]
pub fn agent_enabled_set(id: String, on: bool) -> Result<AgentChoice, RpcError> {
    let store = crate::projects::store()?;
    let mut choice = read(&store);

    choice.disabled.retain(|one| one != &id);
    if !on {
        choice.disabled.push(id.clone());
        if choice.default_id == id {
            store.set_preference(preference::AGENT_DEFAULT, "")?;
        }
    }
    choice.disabled.sort();
    choice.disabled.dedup();

    let written = serde_json::to_string(&choice.disabled)
        .map_err(|err| RpcError::internal(err.to_string()))?;
    store.set_preference(preference::AGENTS_DISABLED, &written)?;
    Ok(read(&store))
}

/// `agent.hooks_set` — whether devpit asks for progress at all.
#[tauri::command]
#[specta::specta]
pub fn agent_hooks_set(on: bool) -> Result<AgentChoice, RpcError> {
    let store = crate::projects::store()?;
    store.set_preference_flag(preference::AGENT_HOOKS, on)?;
    Ok(read(&store))
}

#[cfg(test)]
#[path = "agent_choice_tests.rs"]
mod tests;
