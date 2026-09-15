//! A plugin's files, as the data API hands them out: by name, never by path.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One file in a plugin's data folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginFile {
    /// With its extension, e.g. `flow.excalidraw`.
    pub name: String,
    pub bytes: f64,
    /// Milliseconds since the epoch.
    pub modified: f64,
}

/// Response of `plugin.data.list`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginFiles {
    pub files: Vec<PluginFile>,
}

/// Response of `plugin.data.read`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginFileText {
    pub name: String,
    pub text: String,
    /// Given back on write, so a save can refuse a change it never saw.
    pub modified: f64,
}

/// Response of `plugin.data.write`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginFileSaved {
    pub name: String,
    pub modified: f64,
}

/// Response of `plugin.data.delete`. `false` when there was nothing to remove.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginFileRemoved {
    pub removed: bool,
}
