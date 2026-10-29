//! Profiles another machine made or changed, waiting for the person here
//! before they start: one that arrived and was not yet taken, a secret only
//! this machine may hold, or a change to what runs or where its token goes.

use devpit_core::{Store, PROFILES_WRITE};
use devpit_rpc::{EnvVar, ProfileWaiting, ProfilesWaiting, RpcError};
use devpit_sync::profiles::{keep, kept, BASE, COMMAND, ENV};

/// Every profile that cannot start until the person sees to it.
pub(crate) fn waiting(store: &Store) -> Vec<ProfileWaiting> {
    let kept = kept(store);
    crate::agent_profiles::declared(store)
        .into_iter()
        .filter_map(|one| {
            let held = kept.profiles.get(&one.id)?;
            // Only what arrived empty: a variable left empty on purpose here
            // is the person's choice.
            let needs: Vec<String> = one
                .env
                .iter()
                .filter(|var| held.needs.contains(&var.name) && var.value.is_empty())
                .map(|var| var.name.clone())
                .collect();
            let changed: Vec<EnvVar> = held
                .shadow
                .iter()
                .filter(|(_, shadow)| shadow.ask)
                .map(|(field, shadow)| EnvVar {
                    name: field.clone(),
                    value: shadow.value.clone(),
                })
                .collect();
            (held.arrived || !needs.is_empty() || !changed.is_empty()).then_some(ProfileWaiting {
                id: one.id,
                label: one.label,
                arrived: held.arrived,
                needs,
                changed,
            })
        })
        .collect()
}

/// Why the profile `id` cannot start yet, when it cannot.
pub(crate) fn blocked(store: &Store, id: &str) -> Option<String> {
    let one = waiting(store).into_iter().find(|one| one.id == id)?;
    Some(if one.arrived {
        format!(
            "{} came from another computer — look it over and take it in Settings → Providers first",
            one.label
        )
    } else if let Some(change) = one.changed.first() {
        format!(
            "{}'s {} was changed on another computer — take it or keep this one in Settings → Providers first",
            one.label,
            change.name.trim_start_matches(ENV)
        )
    } else {
        format!(
            "{} needs {} on this computer — fill it in Settings → Providers",
            one.label,
            one.needs.join(", ")
        )
    })
}

/// [`agent_profile_settle`], on a given store: takes what another machine
/// set, or keeps this machine's own. Either way the answer is the person's.
pub(crate) fn settle(store: &Store, id: &str, take: bool) -> Result<(), RpcError> {
    let _held = PROFILES_WRITE
        .lock()
        .map_err(|err| RpcError::internal(err.to_string()))?;
    let mut kept = kept(store);
    let Some(held) = kept.profiles.get_mut(id) else {
        return Ok(());
    };
    let asked: Vec<(String, String)> = held
        .shadow
        .iter()
        .filter(|(_, shadow)| shadow.ask)
        .map(|(field, shadow)| (field.clone(), shadow.value.clone()))
        .collect();
    held.shadow.retain(|_, shadow| !shadow.ask);
    if take {
        held.arrived = false;
        let home = crate::installations::home()?;
        let home = home.to_string_lossy();
        let mut list = crate::agent_profiles::to_edit(store)?;
        if let Some(one) = list.iter_mut().find(|one| one.id == id) {
            for (field, value) in asked {
                match field.as_str() {
                    BASE => one.base = value,
                    COMMAND => one.command = value,
                    field => {
                        let Some(name) = field.strip_prefix(ENV) else {
                            continue;
                        };
                        let value = devpit_agentcli::declaring::at_home_value(&value, &home);
                        match one.env.iter_mut().find(|var| var.name == name) {
                            Some(var) => var.value = value,
                            None => one.env.push(EnvVar {
                                name: name.to_owned(),
                                value,
                            }),
                        }
                    }
                }
            }
        }
        crate::agent_profiles::save(store, &list)?;
    }
    keep(store, &kept).map_err(RpcError::internal)?;
    crate::sync::nudge();
    Ok(())
}

/// `agent.profiles_waiting` — the profiles that wait for the person here.
#[tauri::command]
#[specta::specta]
pub async fn agent_profiles_waiting() -> Result<ProfilesWaiting, RpcError> {
    crate::off_main::blocking(|| {
        Ok(ProfilesWaiting {
            profiles: waiting(&crate::projects::store()?),
        })
    })
    .await
}

/// `agent.profile_settle` — takes what another machine changed in a profile,
/// or one it made, or keeps this machine's own.
#[tauri::command]
#[specta::specta]
pub async fn agent_profile_settle(id: String, take: bool) -> Result<ProfilesWaiting, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        settle(&store, &id, take)?;
        Ok(ProfilesWaiting {
            profiles: waiting(&store),
        })
    })
    .await
}

#[cfg(test)]
#[path = "profile_waiting_tests.rs"]
mod tests;
