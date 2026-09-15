//! Which of the plugins this build ships a project has installed and turned on.

use std::path::Path;

use devpit_core::{home, Store};
use devpit_rpc::{
    catalogue, ErrorCode, PluginList, PluginManifest, PluginState, PluginUninstalled, RpcError,
};

use crate::projects::{home_refusal, store};

/// The manifest for an id, or `NotFound` for one this build does not ship.
pub(crate) fn manifest(plugin_id: &str) -> Result<PluginManifest, RpcError> {
    catalogue()
        .into_iter()
        .find(|one| one.id == plugin_id)
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                format!("no capability {plugin_id:?} in this build"),
            )
        })
}

fn registered(store: &Store, project_id: &str) -> Result<(), RpcError> {
    match store.project(project_id)? {
        Some(_) => Ok(()),
        None => Err(RpcError::new(ErrorCode::NotFound, "no such project")),
    }
}

/// The whole catalogue, each with whether this project installed it and has
/// it on.
pub(crate) fn listed(store: &Store, project_id: &str) -> Result<PluginList, RpcError> {
    registered(store, project_id)?;
    let installed = store.installed_plugins(project_id)?;
    let enabled = store.enabled_plugins(project_id)?;
    let plugins = catalogue()
        .into_iter()
        .map(|manifest| PluginState {
            installed: installed.contains(&manifest.id),
            enabled: enabled.contains(&manifest.id),
            manifest,
        })
        .collect();
    Ok(PluginList { plugins })
}

/// Forbidden rather than invalid when not installed: the request is well
/// formed, and the data API answers a plugin it may not use the same way.
pub(crate) fn set_enabled(
    store: &Store,
    project_id: &str,
    plugin_id: &str,
    enabled: bool,
) -> Result<PluginList, RpcError> {
    let manifest = manifest(plugin_id)?;
    registered(store, project_id)?;
    if !store.set_plugin_enabled(project_id, plugin_id, enabled)? {
        let message = format!("{} is not installed in this project", manifest.name);
        return Err(RpcError::forbidden(message));
    }
    listed(store, project_id)
}

pub(crate) fn install(
    store: &Store,
    project_id: &str,
    plugin_id: &str,
) -> Result<PluginList, RpcError> {
    manifest(plugin_id)?;
    registered(store, project_id)?;
    store.install_plugin(project_id, plugin_id)?;
    listed(store, project_id)
}

pub(crate) fn uninstall(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
    delete_data: bool,
) -> Result<PluginUninstalled, RpcError> {
    manifest(plugin_id)?;
    registered(store, project_id)?;
    let removed_files = home::uninstall_plugin(store, root, project_id, plugin_id, delete_data)
        .map_err(home_refusal)?;
    Ok(PluginUninstalled {
        plugins: listed(store, project_id)?.plugins,
        removed_files,
    })
}

/// `plugin.list` — the catalogue, and what this project has on.
#[tauri::command]
#[specta::specta]
pub fn plugin_list(project_id: String) -> Result<PluginList, RpcError> {
    listed(&store()?, &project_id)
}

/// `plugin.set_enabled` — turns an installed one on or off. Its files stay
/// either way.
#[tauri::command]
#[specta::specta]
pub fn plugin_set_enabled(
    project_id: String,
    plugin_id: String,
    enabled: bool,
) -> Result<PluginList, RpcError> {
    set_enabled(&store()?, &project_id, &plugin_id, enabled)
}

/// `plugin.install` — installs one in this project, on.
#[tauri::command]
#[specta::specta]
pub fn plugin_install(project_id: String, plugin_id: String) -> Result<PluginList, RpcError> {
    install(&store()?, &project_id, &plugin_id)
}

/// `plugin.uninstall` — uninstalls one, deleting its files only when
/// `delete_data`.
#[tauri::command]
#[specta::specta]
pub fn plugin_uninstall(
    project_id: String,
    plugin_id: String,
    delete_data: bool,
) -> Result<PluginUninstalled, RpcError> {
    let (store, root) = (store()?, Store::root()?);
    uninstall(&store, &root, &project_id, &plugin_id, delete_data)
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod tests;
