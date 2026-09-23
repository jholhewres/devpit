//! The installations of the agent CLI this machine has, read off the profiles.
//!
//! One CLI, several configuration directories: a profile that sets
//! `CLAUDE_CONFIG_DIR` runs against its own skills and MCP servers. Skills and
//! MCP ask which one to show, and the answer has to come from here and never
//! from a path the screen sends — this process reads whatever it is pointed at.

use std::path::{Path, PathBuf};

use devpit_agentcli::cli_config::config_dir_from;
use devpit_rpc::{Declared, ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// The variable that moves the CLI's configuration.
const MOVED_BY: &str = "CLAUDE_CONFIG_DIR";

/// The agent whose catalogue this is. Another CLI's skills are not in these
/// directories at all.
const CATALOGUED: &str = "claude";

/// One installation, as the panels offer it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Installation {
    pub directory: String,
    /// The profiles that run against it. Empty for the one this process would
    /// use with no profile at all.
    pub profiles: Vec<String>,
    /// The same profiles by id, which a label is not: two can share a name.
    #[serde(default)]
    pub ids: Vec<String>,
    /// Whether the default profile runs against it — where the panels start.
    pub default: bool,
}

/// An installation with what the readers need and the screen does not.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Found {
    pub directory: PathBuf,
    /// What `CLAUDE_CONFIG_DIR` said for it, if anything. Kept because the
    /// settings file sits beside the default directory and inside a named one.
    pub said: Option<String>,
    pub profiles: Vec<String>,
    pub ids: Vec<String>,
    pub default: bool,
}

/// Every installation, given the profiles and what this process was started
/// with.
///
/// Grouped by directory, because two profiles on one directory — `glm` and a
/// second model on the same account — are one catalogue. A profile pointing
/// at a directory that is not there is left out: offering it would be a list
/// that fails on the click.
pub(crate) fn found_for(
    home: &Path,
    process_said: Option<&str>,
    declared: &[Declared],
    default_id: &str,
    exists: impl Fn(&Path) -> bool,
) -> Vec<Found> {
    let mut all = vec![Found {
        directory: config_dir_from(home, process_said),
        said: process_said.map(ToOwned::to_owned),
        profiles: Vec::new(),
        ids: Vec::new(),
        default: false,
    }];
    for profile in declared.iter().filter(|one| one.base == CATALOGUED) {
        let said = profile
            .env
            .iter()
            .find(|var| var.name == MOVED_BY)
            .map(|var| var.value.as_str())
            .or(process_said);
        let directory = config_dir_from(home, said);
        let default = profile.id == default_id;
        match all.iter_mut().find(|had| had.directory == directory) {
            Some(had) => {
                had.profiles.push(profile.label.clone());
                had.ids.push(profile.id.clone());
                had.default |= default;
            }
            None if exists(&directory) => all.push(Found {
                directory,
                said: said.map(ToOwned::to_owned),
                profiles: vec![profile.label.clone()],
                ids: vec![profile.id.clone()],
                default,
            }),
            None => {}
        }
    }
    // No default profile, or one that is not Claude Code: start where this
    // process would have looked anyway.
    if !all.iter().any(|one| one.default) {
        all[0].default = true;
    }
    all
}

/// The installation asked for, or the default when nothing was.
///
/// Matched against the list, so a directory the screen names is only ever one
/// this module produced.
pub(crate) fn pick(found: Vec<Found>, wanted: Option<&str>) -> Result<Found, RpcError> {
    match wanted {
        None => found
            .into_iter()
            .find(|one| one.default)
            .ok_or_else(|| RpcError::internal("no installation to read")),
        Some(wanted) => found
            .into_iter()
            .find(|one| one.directory == Path::new(wanted))
            .ok_or_else(|| {
                RpcError::new(
                    ErrorCode::Forbidden,
                    "that is not an installation any profile runs against",
                )
            }),
    }
}

pub(crate) fn home() -> Result<PathBuf, RpcError> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| RpcError::internal("there is no HOME to read the CLI's config from"))
}

/// The installations on this machine now.
pub(crate) fn found() -> Result<Vec<Found>, RpcError> {
    let store = crate::projects::store()?;
    Ok(found_for(
        &home()?,
        std::env::var(MOVED_BY).ok().as_deref(),
        &crate::agent_profiles::declared(&store),
        &crate::agent_choice::default_id(&store),
        Path::exists,
    ))
}

/// The installation a panel asked for.
pub(crate) fn chosen(wanted: Option<&str>) -> Result<Found, RpcError> {
    pick(found()?, wanted)
}

/// `cli.installations` — what the Skills and MCP panels can switch between.
#[tauri::command]
#[specta::specta]
pub fn cli_installations() -> Result<Vec<Installation>, RpcError> {
    Ok(found()?
        .into_iter()
        .map(|one| Installation {
            directory: one.directory.display().to_string(),
            profiles: one.profiles,
            ids: one.ids,
            default: one.default,
        })
        .collect())
}

#[cfg(test)]
#[path = "installations_tests.rs"]
mod tests;
