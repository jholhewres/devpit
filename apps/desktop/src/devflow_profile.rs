//! The account Devflow's reviewers run as in a project.
//!
//! A reviewer is the person's own CLI, headless. Started bare it took the
//! machine's default account, whatever account the project's sessions use,
//! so a personal project's reviews ran on the company's account (#249).

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::{Declared, DevflowProfile, DevflowProfileChoice, ErrorCode, RpcError};

fn key(project_id: &str) -> String {
    format!("project.profile:{project_id}")
}

/// The profile `project_id` names for its reviewers, if it names one.
pub(crate) fn named(store: &Store, project_id: &str) -> Option<String> {
    store
        .preference(&key(project_id))
        .ok()
        .flatten()
        .filter(|id| !id.is_empty())
}

/// Names the project's profile; `None` goes back to the rule in [`chosen`].
pub(crate) fn set_named(
    store: &Store,
    project_id: &str,
    profile: Option<&str>,
) -> Result<(), String> {
    store
        .set_preference(&key(project_id), profile.unwrap_or_default())
        .map_err(|err| err.to_string())
}

/// What decides the profile.
#[derive(Clone, Copy)]
pub(crate) struct Asked<'a> {
    /// The one the project names.
    pub named: Option<&'a str>,
    /// The `CLAUDE_CONFIG_DIR` of the session that asked, when it had one.
    pub asker_config: Option<&'a Path>,
    /// The profile of each orchestrator the project is linked to.
    pub linked: &'a [String],
}

fn same_dir(a: &Path, b: &Path) -> bool {
    a.components().eq(b.components())
}

/// The profile the reviewers run as: the project's own, else the account of
/// the session that asked, else the one every orchestrator linked to the
/// project speaks as. `None` is the machine's default, as before.
pub(crate) fn chosen<'a>(declared: &'a [Declared], asked: &Asked) -> Option<&'a Declared> {
    let by_id = |id: &str| declared.iter().find(|one| one.id == id);
    if let Some(named) = asked.named {
        return by_id(named);
    }
    if let Some(asker) = asked.asker_config {
        let mut same = declared.iter().filter(|one| {
            one.env
                .iter()
                .rev()
                .find(|set| set.name == "CLAUDE_CONFIG_DIR")
                .is_some_and(|set| same_dir(Path::new(&set.value), asker))
        });
        if let (Some(one), None) = (same.next(), same.next()) {
            return Some(one);
        }
    }
    let first = asked.linked.first()?;
    asked
        .linked
        .iter()
        .all(|one| one == first)
        .then(|| by_id(first))
        .flatten()
}

/// The profile's environment, for a reviewer: only a Claude profile's, since
/// the reviewers it would change are Claude Code's.
pub(crate) fn env_of(profile: Option<&Declared>) -> Vec<(String, String)> {
    profile
        .filter(|one| one.base == "claude")
        .map(|one| {
            one.env
                .iter()
                .map(|set| (set.name.clone(), set.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// Where the reviewers' Claude Code keeps its sign-in, `inherited` being
/// this process's `CLAUDE_CONFIG_DIR`, which a profile's own replaces.
pub(crate) fn config_dir(
    profile: Option<&Declared>,
    home: &Path,
    inherited: Option<&str>,
) -> PathBuf {
    devpit_agentcli::cli_config::config_dir_of(home, &env_of(profile), inherited)
}

/// The profile of every orchestrator linked to `project_id`.
fn linked(store: &Store, project_id: &str) -> Vec<String> {
    let Ok(root) = Store::root() else {
        return Vec::new();
    };
    store
        .projects()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| {
            let folder = PathBuf::from(&row.root_path);
            let profile = devpit_core::home::orchestrator_of(store, &root, &folder)?;
            crate::orchestrator_links::linked_in(store, &folder)
                .iter()
                .any(|one| one == project_id)
                .then_some(profile)
        })
        .collect()
}

/// The profile `project_id`'s reviewers run as, asked from a session whose
/// `CLAUDE_CONFIG_DIR` is `asker_config`.
pub(crate) fn of(store: &Store, project_id: &str, asker_config: Option<&Path>) -> Option<Declared> {
    let named = named(store, project_id);
    let linked = linked(store, project_id);
    let declared = crate::agent_profiles::declared(store);
    chosen(
        &declared,
        &Asked {
            named: named.as_deref(),
            asker_config,
            linked: &linked,
        },
    )
    .cloned()
}

/// What the Devflow screen says about the account the reviewers run as.
pub(crate) fn shown(store: &Store, project_id: &str) -> DevflowProfile {
    let named = named(store, project_id);
    let runs_as = of(store, project_id, None);
    let from = match (&named, &runs_as) {
        (Some(_), Some(_)) => "project",
        (None, Some(_)) => "orchestrator",
        _ => "default",
    };
    DevflowProfile {
        project_id: project_id.to_owned(),
        named,
        runs_as: runs_as.map(|one| one.label),
        from: from.to_owned(),
        choices: crate::agent_profiles::declared(store)
            .into_iter()
            .filter(|one| one.base == "claude")
            .map(|one| DevflowProfileChoice {
                id: one.id,
                label: one.label,
            })
            .collect(),
    }
}

/// `devflow.profile` — the account a project's Claude Code reviewers run as.
#[tauri::command]
#[specta::specta]
pub async fn devflow_profile(project_id: String) -> Result<DevflowProfile, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        Ok(shown(&store, &project_id))
    })
    .await
}

/// `devflow.profile_set` — the profile the project's reviewers run as, by
/// id; `None` goes back to the orchestrator's or the machine's default.
#[tauri::command]
#[specta::specta]
pub async fn devflow_profile_set(
    project_id: String,
    profile_id: Option<String>,
) -> Result<DevflowProfile, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        crate::projects::locate(&store, &project_id)?;
        let wanted = profile_id.filter(|id| !id.is_empty());
        if let Some(id) = &wanted {
            let known = crate::agent_profiles::declared(&store)
                .iter()
                .any(|one| &one.id == id && one.base == "claude");
            if !known {
                return Err(RpcError::new(
                    ErrorCode::Invalid,
                    "no Claude profile by that id on this machine",
                ));
            }
        }
        set_named(&store, &project_id, wanted.as_deref())
            .map_err(|why| RpcError::new(ErrorCode::Internal, why))?;
        Ok(shown(&store, &project_id))
    })
    .await
}

#[cfg(test)]
#[path = "devflow_profile_tests.rs"]
mod tests;
