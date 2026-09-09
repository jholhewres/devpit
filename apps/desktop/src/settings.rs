//! Contract commands for what the person chose.

use devpit_core::{preference, Store};
use devpit_rpc::{RpcError, Settings, Theme};

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

fn read(store: &Store) -> Result<Settings, RpcError> {
    Ok(Settings {
        telemetry: store.preference_flag(preference::TELEMETRY)?,
        theme: store
            .preference(preference::THEME)?
            .map_or(Theme::Dark, |stored| Theme::parse(&stored)),
        // Filled in when there is an account to be signed into. Absent rather
        // than an empty string: "not signed in" and "signed in as nobody" are
        // different, and only one of them is real.
        account: None,
        onboarded_at: store
            .preference(preference::ONBOARDED_AT)?
            .and_then(|value| value.parse::<i64>().ok())
            .map(|seconds| seconds as f64),
        automatic_updates: store.preference_flag(preference::AUTO_UPDATE)?,
        keep_transcripts: store.preference_flag(preference::KEEP_TRANSCRIPTS)?,
    })
}

/// `settings.read` — everything the first run and the settings screen need.
#[tauri::command]
#[specta::specta]
pub fn settings_read() -> Result<Settings, RpcError> {
    read(&store()?)
}

/// `settings.write` — records a choice and answers with the whole object.
///
/// Answering with the whole object rather than nothing means the screen never
/// has to predict what a write did to the rest of it.
#[tauri::command]
#[specta::specta]
pub fn settings_write(
    telemetry: Option<bool>,
    theme: Option<Theme>,
    automatic_updates: Option<bool>,
    keep_transcripts: Option<bool>,
) -> Result<Settings, RpcError> {
    let store = store()?;
    if let Some(allowed) = telemetry {
        store.set_preference_flag(preference::TELEMETRY, allowed)?;
    }
    if let Some(chosen) = theme {
        store.set_preference(preference::THEME, chosen.stored())?;
    }
    if let Some(automatic) = automatic_updates {
        store.set_preference_flag(preference::AUTO_UPDATE, automatic)?;
    }
    if let Some(keep) = keep_transcripts {
        store.set_preference_flag(preference::KEEP_TRANSCRIPTS, keep)?;
    }
    read(&store)
}

/// `settings.finish_onboarding` — the first run is done.
///
/// Recorded so the flow does not reappear for someone who finished it and then
/// removed their only project — those are different situations and the second
/// one is not a first run.
#[tauri::command]
#[specta::specta]
pub fn settings_finish_onboarding() -> Result<Settings, RpcError> {
    let store = store()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    store.set_preference(preference::ONBOARDED_AT, &now.to_string())?;
    read(&store)
}
