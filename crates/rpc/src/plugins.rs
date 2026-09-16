//! The plugin contract: what a first-party plugin declares about itself, and
//! the catalogue of plugins this build ships.
//!
//! A plugin's data lives as files in its own folder inside a project;
//! whether it is enabled is a per-project choice made elsewhere. This module
//! carries only the shape of a manifest and the rules one must satisfy.
//!
//! Lives in `devpit-rpc`, and the catalogue and validation stay beside the
//! types rather than moving to `devpit-core`: `devpit-rpc` already depends on
//! `devpit-core`, so a type `core` could see would make `core` depend on
//! `rpc` right back — the cycle `cargo` refuses. `specta::Type` is also only
//! ever a dependency of this crate.

use serde::{Deserialize, Serialize};
use specta::Type;

/// The ceiling every plugin's data folder is allowed to declare. 32 MiB is
/// enough for the diagrams the first plugin, Excalidraw, produces, without
/// handing a plugin the run of the disk.
pub const MAX_DATA_BYTES: f64 = (32 * 1024 * 1024) as f64;

/// One capability of the desktop UI a plugin can occupy.
///
/// Tagged the way [`crate::session::LayoutNode`] is: `Pane` carries data,
/// `CardPin` does not, and one enum says both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Surface {
    /// A pane in the workspace strip. `many` says whether a project may have
    /// more than one open at once.
    Pane { many: bool },
    /// A pin shown on the card face.
    CardPin,
}

/// What a plugin's data folder is allowed to hold.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataSpec {
    /// Each with its leading dot, e.g. `.excalidraw`. Enforced by [`validate`].
    pub extensions: Vec<String>,
    /// Bytes; `f64` and not `u64` because this crosses into a JavaScript
    /// number and specta refuses `u64` by default — see `Commit::committed_at`
    /// for why `f64`.
    pub max_bytes: f64,
}

/// What a plugin is allowed to do beyond drawing its own surfaces.
///
/// A closed set on purpose: a plugin compiled into the app is trusted with
/// exactly what this enum names, never with whatever it asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Permission {
    /// Owns the files under its own data folder — nothing outside it.
    DataOwn,
}

/// What one plugin declares about itself.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    /// Stable, and the name of its data folder. Checked by [`validate`].
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub surfaces: Vec<Surface>,
    pub data: DataSpec,
    pub permissions: Vec<Permission>,
}

/// One plugin of the catalogue, whether this project installed it, and
/// whether it is on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginState {
    pub manifest: PluginManifest,
    /// Only an installed plugin can be on.
    pub installed: bool,
    pub enabled: bool,
}

/// Response of `plugin.list`, `plugin.install` and `plugin.set_enabled`: the
/// whole catalogue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginList {
    pub plugins: Vec<PluginState>,
}

/// Response of `plugin.uninstall`: the catalogue as it is now, under the same
/// field as [`PluginList`], and how many of the plugin's files were deleted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PluginUninstalled {
    pub plugins: Vec<PluginState>,
    /// 0 when the files were kept.
    pub removed_files: u32,
}

/// Why a manifest, or the catalogue as a whole, was refused.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginError {
    InvalidId { id: String },
    InvalidExtension { plugin: String, extension: String },
    InvalidMaxBytes { plugin: String, max_bytes: f64 },
    DuplicateId { id: String },
}

impl std::fmt::Display for PluginError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidId { id } => {
                write!(f, "plugin id {id:?} must match ^[a-z][a-z0-9-]{{1,31}}$")
            }
            Self::InvalidExtension { plugin, extension } => write!(
                f,
                "plugin {plugin:?} extension {extension:?} must be '.' then letters, digits, '-' or '_'"
            ),
            Self::InvalidMaxBytes { plugin, max_bytes } => write!(
                f,
                "plugin {plugin:?} max_bytes {max_bytes} must be a whole number from 1 to {MAX_DATA_BYTES}"
            ),
            Self::DuplicateId { id } => write!(f, "duplicate plugin id {id:?} in the catalogue"),
        }
    }
}

impl std::error::Error for PluginError {}

/// `.` then at least one of `[A-Za-z0-9_-]`: an extension is matched against
/// the end of a file name, so it carries nothing a name may not.
fn extension_is_well_formed(extension: &str) -> bool {
    extension.strip_prefix('.').is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .chars()
                .all(|letter| letter.is_ascii_alphanumeric() || letter == '-' || letter == '_')
    })
}

/// A whole number of bytes from 1 to [`MAX_DATA_BYTES`]. Asked as a range
/// rather than refusing the excess, because NaN fails every comparison.
fn max_bytes_is_well_formed(max_bytes: f64) -> bool {
    max_bytes.is_finite() && max_bytes.fract() == 0.0 && (1.0..=MAX_DATA_BYTES).contains(&max_bytes)
}

/// Refuses a manifest whose id, extensions or data ceiling break the
/// contract. Does not compare across manifests — that is [`validate_catalogue`].
pub fn validate(manifest: &PluginManifest) -> Result<(), PluginError> {
    // The id names the data folder, so the folder's rule is the id's rule.
    if !devpit_core::home::plain_plugin_id(&manifest.id) {
        return Err(PluginError::InvalidId {
            id: manifest.id.clone(),
        });
    }
    for extension in &manifest.data.extensions {
        if !extension_is_well_formed(extension) {
            return Err(PluginError::InvalidExtension {
                plugin: manifest.id.clone(),
                extension: extension.clone(),
            });
        }
    }
    if !max_bytes_is_well_formed(manifest.data.max_bytes) {
        return Err(PluginError::InvalidMaxBytes {
            plugin: manifest.id.clone(),
            max_bytes: manifest.data.max_bytes,
        });
    }
    Ok(())
}

/// Validates every manifest, then refuses the set if two share an id — the
/// name of the data folder a plugin owns, which two plugins cannot both be.
pub fn validate_catalogue(manifests: &[PluginManifest]) -> Result<(), PluginError> {
    for manifest in manifests {
        validate(manifest)?;
    }
    let mut seen = std::collections::HashSet::new();
    for manifest in manifests {
        if !seen.insert(manifest.id.as_str()) {
            return Err(PluginError::DuplicateId {
                id: manifest.id.clone(),
            });
        }
    }
    Ok(())
}

/// The plugins this build ships, compiled in. Enabling one is a per-project
/// choice made elsewhere; this only says what exists.
pub fn catalogue() -> Vec<PluginManifest> {
    vec![excalidraw(), mermaid(), notes(), data()]
}

fn excalidraw() -> PluginManifest {
    PluginManifest {
        id: "excalidraw".to_owned(),
        name: "Excalidraw".to_owned(),
        version: "0.1.0".to_owned(),
        description: "Hand-drawn diagrams, saved as files in the project.".to_owned(),
        surfaces: vec![Surface::Pane { many: true }, Surface::CardPin],
        data: DataSpec {
            extensions: vec![".excalidraw".to_owned()],
            max_bytes: MAX_DATA_BYTES,
        },
        permissions: vec![Permission::DataOwn],
    }
}

/// Diagrams written as text, drawn by the renderer the window already has.
fn mermaid() -> PluginManifest {
    PluginManifest {
        id: "mermaid".to_owned(),
        name: "Mermaid".to_owned(),
        version: "0.1.0".to_owned(),
        description: "Diagrams written as text, saved as files in the project.".to_owned(),
        surfaces: vec![Surface::Pane { many: true }, Surface::CardPin],
        data: DataSpec {
            extensions: vec![".mmd".to_owned()],
            max_bytes: MAX_DATA_BYTES,
        },
        permissions: vec![Permission::DataOwn],
    }
}

/// Notes in Markdown, drawn by the renderer the window already has.
fn notes() -> PluginManifest {
    PluginManifest {
        id: "notes".to_owned(),
        name: "Notes".to_owned(),
        version: "0.1.0".to_owned(),
        description: "Notes in Markdown, saved as files in the project.".to_owned(),
        surfaces: vec![Surface::Pane { many: true }, Surface::CardPin],
        data: DataSpec {
            extensions: vec![".md".to_owned()],
            max_bytes: MAX_DATA_BYTES,
        },
        permissions: vec![Permission::DataOwn],
    }
}

/// JSON and YAML, drawn as a graph rather than read as a wall of braces.
fn data() -> PluginManifest {
    PluginManifest {
        id: "data".to_owned(),
        name: "Data".to_owned(),
        version: "0.1.0".to_owned(),
        description: "JSON and YAML, drawn as a graph you can walk.".to_owned(),
        surfaces: vec![Surface::Pane { many: true }, Surface::CardPin],
        data: DataSpec {
            extensions: vec![".json".to_owned(), ".yaml".to_owned(), ".yml".to_owned()],
            max_bytes: MAX_DATA_BYTES,
        },
        permissions: vec![Permission::DataOwn],
    }
}

#[cfg(test)]
#[path = "plugins_tests.rs"]
mod tests;
