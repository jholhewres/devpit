//! The account Devflow's reviewers run as in a project.
//!
//! A reviewer is the person's own CLI, headless. Started bare it took the
//! machine's default account, whatever account the project's sessions use,
//! so a personal project's reviews ran on the company's account (#249).

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use devpit_core::preference::DEVFLOW_PROFILES;
use devpit_core::Store;
use devpit_rpc::{
    Declared, DevflowProfile, DevflowProfileChoice, DevflowProfileFrom, ErrorCode, Profile,
    RpcError,
};
use serde_json::{Map, Value};

fn all_named(store: &Store) -> Map<String, Value> {
    store
        .preference(DEVFLOW_PROFILES)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

/// The profile `project_id` names for its reviewers, if it names one.
pub(crate) fn named(store: &Store, project_id: &str) -> Option<String> {
    all_named(store)
        .get(project_id)
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_owned)
}

/// Names the project's profile; `None` goes back to the rule in [`chosen`].
pub(crate) fn set_named(
    store: &Store,
    project_id: &str,
    profile: Option<&str>,
) -> Result<(), String> {
    // One map for every project, read and written whole: two at once would
    // each write back a map without the other's change.
    static NAMING: Mutex<()> = Mutex::new(());
    let _one_at_a_time = NAMING.lock();
    let mut all = all_named(store);
    match profile {
        Some(id) => all.insert(project_id.to_owned(), Value::from(id)),
        None => all.remove(project_id),
    };
    store
        .set_preference(DEVFLOW_PROFILES, &Value::Object(all).to_string())
        .map_err(|err| err.to_string())
}

/// Whether a profile changes the reviewers: only a Claude one does, since the
/// reviewers it would start are Claude Code's.
fn reviews(base: &str) -> bool {
    base == "claude"
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
    // A named profile since deleted names nothing.
    if let Some(named) = asked.named.and_then(by_id) {
        return Some(named);
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
        .filter(|one| reviews(&one.base))
        .map(|one| {
            one.env
                .iter()
                .map(|set| (set.name.clone(), set.value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

/// The program a Claude profile starts, resolved to its path, when devpit can
/// spawn it: a shell function, or a command that resolves nowhere, leaves the
/// reviewer on the plain CLI with the profile's environment.
pub(crate) fn program_from(profile: &Profile) -> Option<String> {
    // `path` is set only when the command resolves to a file.
    reviews(&profile.base)
        .then(|| profile.path.clone())
        .flatten()
}

/// [`program_from`], for a declared profile of this machine.
pub(crate) fn program_of(store: &Store, profile: &Declared) -> Option<String> {
    crate::agent_profiles::all(store)
        .ok()?
        .iter()
        .find(|one| one.id == profile.id)
        .and_then(program_from)
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
    // Only a Claude profile changes the reviewers, so only one is chosen.
    let declared: Vec<Declared> = crate::agent_profiles::declared(store)
        .into_iter()
        .filter(|one| reviews(&one.base))
        // One that waits for the person here runs nothing, reviewers included.
        .filter(|one| crate::profile_waiting::blocked(store, &one.id).is_none())
        .collect();
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
    let runs_as = of(store, project_id, None);
    // A named profile since deleted names nothing.
    let named =
        named(store, project_id).filter(|id| runs_as.as_ref().is_some_and(|one| &one.id == id));
    let from = match (&named, &runs_as) {
        (Some(_), _) => DevflowProfileFrom::Project,
        (None, Some(_)) => DevflowProfileFrom::Orchestrator,
        (None, None) => DevflowProfileFrom::Default,
    };
    DevflowProfile {
        project_id: project_id.to_owned(),
        named,
        runs_as: runs_as.map(|one| one.label),
        from,
        choices: crate::agent_profiles::declared(store)
            .into_iter()
            .filter(|one| reviews(&one.base))
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
                .any(|one| &one.id == id && reviews(&one.base));
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
