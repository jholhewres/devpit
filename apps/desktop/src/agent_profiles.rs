//! The agent profiles a person declared, and the list they end up seeing.
//!
//! A profile is environment, program and arguments — the shape five of these
//! already had as shell functions before there was anywhere to put them. What
//! this module owns is the round trip: where they are stored, what is refused
//! on the way in, and how a stored one becomes a row with a resolved path.
//!
//! Stored as one JSON preference rather than a table, for the same reason the
//! "Open in" apps are: it is a short list edited by hand, and a table would be
//! a migration for every field somebody wants next. The cost is honest — there
//! is no foreign key, so a step naming a profile that was deleted holds a
//! string pointing at nothing, which is why deleting asks first.

use devpit_agentcli::profile::{profiles, Base};
use devpit_core::store::preference;
use devpit_core::Store;
use devpit_rpc::{Declared, ErrorCode, Profile, RpcError};

/// What a base agent lends, looked up in the agent catalogue.
fn base_of(id: &str) -> Option<Base> {
    devpit_pty::agents::known(id).map(|agent| Base {
        program: agent.launch.to_owned(),
        driver: agent.driver.to_owned(),
    })
}

fn stored(store: &Store) -> Result<Vec<Declared>, RpcError> {
    let raw = store
        .preference(preference::AGENT_PROFILES)?
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    // A preference that cannot be parsed is an empty list, not an error: it is
    // one row in a settings table, and refusing to draw the pane over it would
    // make a typo unrecoverable from inside the app.
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

pub(crate) fn save(store: &Store, declared: &[Declared]) -> Result<(), RpcError> {
    let written =
        serde_json::to_string(declared).map_err(|err| RpcError::internal(err.to_string()))?;
    store.set_preference(preference::AGENT_PROFILES, &written)?;
    Ok(())
}

/// What the person called each profile, by id.
///
/// Deliberately not `all`: the sidebar asks this on a two-second timer and
/// wants two strings, while `all` resolves every command against the `PATH`.
/// That is a directory walk per profile per tick for an answer nobody read.
pub(crate) fn names(store: &Store) -> std::collections::HashMap<String, String> {
    stored(store)
        .unwrap_or_default()
        .into_iter()
        .map(|one| (one.id, one.label))
        .collect()
}

/// The profiles the person declared, as written.
///
/// Not `all`: the installations need only the environment, and `all` resolves
/// every command against the `PATH` and asks the shell about the rest.
pub(crate) fn declared(store: &Store) -> Vec<Declared> {
    stored(store).unwrap_or_default()
}

/// The command each declared profile runs, for the shell probe.
pub(crate) fn commands(store: &Store) -> Vec<String> {
    stored(store)
        .unwrap_or_default()
        .into_iter()
        .filter(|one| !one.command.is_empty())
        .map(|one| one.command)
        .collect()
}

/// Every profile this machine can offer: the declared ones and the discovered.
pub(crate) fn all(store: &Store) -> Result<Vec<Profile>, RpcError> {
    Ok(profiles(
        &stored(store)?,
        base_of,
        crate::shell_launch::shell_knows(),
    ))
}

/// `agent.profiles` — the accounts this machine can talk to.
#[tauri::command]
#[specta::specta]
pub fn agent_profiles() -> Result<Vec<Profile>, RpcError> {
    all(&crate::projects::store()?)
}

/// `agent.profile_save` — writes one profile, new or edited.
///
/// The id decides which: an id already on the list is an edit in place, so a
/// rename keeps every step that names it. A blank one is minted here rather
/// than in the window, because the window can be reloaded mid-edit.
#[tauri::command]
#[specta::specta]
pub fn agent_profile_save(declared: Declared) -> Result<Vec<Profile>, RpcError> {
    devpit_agentcli::declaring::allowed(&declared, |base| base_of(base).is_some())
        .map_err(|why| RpcError::new(ErrorCode::Invalid, why.to_string()))?;
    let home = crate::installations::home()?;
    let mut declared = devpit_agentcli::declaring::at_home(declared, &home.to_string_lossy());

    declared.label = declared.label.trim().to_owned();
    if declared.id.trim().is_empty() {
        declared.id = ulid::Ulid::generate().to_string();
    }

    let store = crate::projects::store()?;
    let mut list = stored(&store)?;
    match list.iter_mut().find(|one| one.id == declared.id) {
        Some(existing) => *existing = declared,
        None => list.push(declared),
    }
    save(&store, &list)?;
    all(&store)
}

/// `agent.profile_remove` — forgets one.
///
/// Refused while a board step still names it. A step holds the id as a plain
/// string with nothing enforcing it, so a silent delete is a lane that fails
/// the next time somebody plays a card — and by then the deletion is days ago
/// and nowhere near the symptom.
#[tauri::command]
#[specta::specta]
pub fn agent_profile_remove(id: String) -> Result<Vec<Profile>, RpcError> {
    let store = crate::projects::store()?;
    let used = store.steps_using_profile(&id)?;
    if !used.is_empty() {
        // Named, with their boards: "a step uses it" on a machine with six
        // projects tells somebody there is a problem and not where it is.
        let named: Vec<String> = used
            .iter()
            .map(|one| format!("{} in {}", one.step, one.project))
            .collect();
        return Err(RpcError::new(
            ErrorCode::Conflict,
            format!(
                "{} still {} this profile: {}",
                if used.len() == 1 {
                    "a step".to_owned()
                } else {
                    format!("{} steps", used.len())
                },
                if used.len() == 1 { "uses" } else { "use" },
                named.join(", ")
            ),
        ));
    }

    let mut list = stored(&store)?;
    list.retain(|one| one.id != id);
    save(&store, &list)?;
    all(&store)
}

/// The profile this step names, as something that can be started.
pub(crate) fn runner_for(
    store: &Store,
    id: &str,
) -> Result<devpit_agentcli::running::Runner, String> {
    let found = all(store)
        .map_err(|err| err.to_string())?
        .into_iter()
        .find(|one| one.id == id)
        .ok_or_else(|| format!("this step runs under a profile that no longer exists: {id}"))?;
    if !found.spawnable() {
        // A shell function has nothing to spawn, and saying so here beats a
        // process that fails to start with the operating system's word for it.
        return Err(format!(
            "{} cannot be started outside a terminal — name the program it runs",
            found.label
        ));
    }
    Ok(devpit_agentcli::running::runner(&found))
}

#[cfg(test)]
#[path = "agent_profiles_tests.rs"]
mod tests;
