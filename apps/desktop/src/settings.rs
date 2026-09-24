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
        focus_mode: store.preference_flag(preference::FOCUS_MODE)?,
        error_reports: store.preference_flag(preference::ERROR_REPORTS)?,
    })
}

/// `settings.read` — everything the first run and the settings screen need.
#[tauri::command]
#[specta::specta]
pub async fn settings_read() -> Result<Settings, RpcError> {
    crate::off_main::blocking(settings_read_now).await
}

/// [`settings_read`], on the calling thread.
pub(crate) fn settings_read_now() -> Result<Settings, RpcError> {
    read(&store()?)
}

/// `settings.write` — records a choice and answers with the whole object.
///
/// Answering with the whole object rather than nothing means the screen never
/// has to predict what a write did to the rest of it.
#[tauri::command]
#[specta::specta]
pub async fn settings_write(
    theme: Option<Theme>,
    automatic_updates: Option<bool>,
    confirm_stop: Option<bool>,
    terminal_contrast: Option<f64>,
    focus_mode: Option<bool>,
    error_reports: Option<bool>,
) -> Result<Settings, RpcError> {
    crate::off_main::blocking(move || {
        settings_write_now(
            theme,
            automatic_updates,
            confirm_stop,
            terminal_contrast,
            focus_mode,
            error_reports,
        )
    })
    .await
}

/// [`settings_write`], on the calling thread.
pub(crate) fn settings_write_now(
    theme: Option<Theme>,
    automatic_updates: Option<bool>,
    confirm_stop: Option<bool>,
    terminal_contrast: Option<f64>,
    focus_mode: Option<bool>,
    error_reports: Option<bool>,
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
    if let Some(offered) = focus_mode {
        store.set_preference_flag(preference::FOCUS_MODE, offered)?;
    }
    if let Some(kept) = error_reports {
        // Now, not at the next start: off means nothing kept from this moment.
        crate::error_reports::choose(&store, &devpit_core::Store::root()?, kept)?;
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
pub async fn settings_finish_onboarding() -> Result<Settings, RpcError> {
    crate::off_main::blocking(settings_finish_onboarding_now).await
}

/// [`settings_finish_onboarding`], on the calling thread.
pub(crate) fn settings_finish_onboarding_now() -> Result<Settings, RpcError> {
    let store = store()?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    store.set_preference(preference::ONBOARDED_AT, &now.to_string())?;
    read(&store)
}
