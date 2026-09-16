//! Contract commands for what the person chose.

use devpit_core::{preference, Store};
use devpit_rpc::{ErrorCode, RpcError, Settings, Theme};

/// What xterm accepts: 1 is no lift, 21 is black on white.
const CONTRAST: std::ops::RangeInclusive<f64> = 1.0..=21.0;

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

fn read(store: &Store) -> Result<Settings, RpcError> {
    Ok(Settings {
        theme: store
            .preference(preference::THEME)?
            .map_or(Theme::Dark, |stored| Theme::parse(&stored)),
        onboarded_at: store
            .preference(preference::ONBOARDED_AT)?
            .and_then(|value| value.parse::<i64>().ok())
            .map(|seconds| seconds as f64),
        automatic_updates: store.preference_flag(preference::AUTO_UPDATE)?,
        confirm_stop: store.preference_flag(preference::CONFIRM_STOP)?,
        terminal_contrast: store
            .preference(preference::TERMINAL_CONTRAST)?
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| CONTRAST.contains(value)),
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
    theme: Option<Theme>,
    automatic_updates: Option<bool>,
    confirm_stop: Option<bool>,
    terminal_contrast: Option<f64>,
) -> Result<Settings, RpcError> {
    let store = store()?;
    if let Some(chosen) = theme {
        store.set_preference(preference::THEME, chosen.stored())?;
    }
    if let Some(automatic) = automatic_updates {
        store.set_preference_flag(preference::AUTO_UPDATE, automatic)?;
    }
    if let Some(ask) = confirm_stop {
        store.set_preference_flag(preference::CONFIRM_STOP, ask)?;
    }
    if let Some(contrast) = terminal_contrast {
        if !CONTRAST.contains(&contrast) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "terminal contrast goes from 1 to 21",
            ));
        }
        store.set_preference(preference::TERMINAL_CONTRAST, &contrast.to_string())?;
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
