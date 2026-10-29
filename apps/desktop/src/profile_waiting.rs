//! Profiles another machine made or changed, waiting for the person here
//! before they start: a secret only this machine may hold, or a base URL
//! changed elsewhere that would send the profile's token somewhere new.

use std::collections::BTreeMap;
use std::path::Path;

use devpit_core::{preference, Store};
use devpit_rpc::{EnvVar, ProfileWaiting, RpcError};
use devpit_sync::profiles::{shared, Profiles};

type Pending = BTreeMap<String, BTreeMap<String, String>>;

fn pending(store: &Store) -> Pending {
    Profiles {
        store,
        home: Path::new(""),
    }
    .pending()
    .unwrap_or_default()
}

/// Every profile that cannot start until the person sees to it.
pub(crate) fn waiting(store: &Store) -> Vec<ProfileWaiting> {
    let pending = pending(store);
    crate::agent_profiles::declared(store)
        .into_iter()
        .filter_map(|one| {
            let needs: Vec<String> = one
                .env
                .iter()
                .filter(|var| !shared(&var.name) && var.value.is_empty())
                .map(|var| var.name.clone())
                .collect();
            let changed: Vec<EnvVar> = pending
                .get(&one.id)
                .map(|names| {
                    names
                        .iter()
                        .map(|(name, value)| EnvVar {
                            name: name.clone(),
                            value: value.clone(),
                        })
                        .collect()
                })
                .unwrap_or_default();
            (!needs.is_empty() || !changed.is_empty()).then_some(ProfileWaiting {
                id: one.id,
                label: one.label,
                needs,
                changed,
            })
        })
        .collect()
}

/// Why the profile `id` cannot start yet, when it cannot.
pub(crate) fn blocked(store: &Store, id: &str) -> Option<String> {
    let one = waiting(store).into_iter().find(|one| one.id == id)?;
    Some(match one.changed.first() {
        Some(var) => format!(
            "{}'s {} was changed on another computer — take it or keep this one in Settings → Providers first",
            one.label, var.name
        ),
        None => format!(
            "{} needs {} on this computer — fill it in Settings → Providers",
            one.label,
            one.needs.join(", ")
        ),
    })
}

/// [`agent_profile_settle`], on a given store.
pub(crate) fn settle(store: &Store, id: &str, take: bool) -> Result<(), RpcError> {
    let mut all = pending(store);
    let Some(changed) = all.remove(id) else {
        return Ok(());
    };
    if take {
        let mut list = crate::agent_profiles::to_edit(store)?;
        if let Some(one) = list.iter_mut().find(|one| one.id == id) {
            for (name, value) in &changed {
                match one.env.iter_mut().find(|var| &var.name == name) {
                    Some(var) => var.value = value.clone(),
                    None => one.env.push(EnvVar {
                        name: name.clone(),
                        value: value.clone(),
                    }),
                }
            }
        }
        crate::agent_profiles::save(store, &list)?;
    }
    // Kept or taken, the answer is the person's: the next round sends it.
    store.set_preference(
        preference::AGENT_PROFILES_PENDING,
        &serde_json::to_string(&all).unwrap_or_else(|_| "{}".to_owned()),
    )?;
    crate::sync::nudge();
    Ok(())
}

/// `agent.profiles_waiting` — the profiles that wait for the person here.
#[tauri::command]
#[specta::specta]
pub async fn agent_profiles_waiting() -> Result<Vec<ProfileWaiting>, RpcError> {
    crate::off_main::blocking(|| Ok(waiting(&crate::projects::store()?))).await
}

/// `agent.profile_settle` — takes what another machine changed in a profile,
/// or keeps this machine's own.
#[tauri::command]
#[specta::specta]
pub async fn agent_profile_settle(id: String, take: bool) -> Result<Vec<ProfileWaiting>, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        settle(&store, &id, take)?;
        Ok(waiting(&store))
    })
    .await
}

#[cfg(test)]
#[path = "profile_waiting_tests.rs"]
mod tests;
