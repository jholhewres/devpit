//! What a desktop app is expected to have: one copy running, a place in the
//! tray (`tray`), opening at login, and a shortcut that brings it forward
//! from anywhere.
//!
//! One copy, because a second one is not harmless: it opens its own hook
//! listener and its own agent door, writes their endpoint and secret over the
//! first one's, and every session's hooks and MCP calls then post to a port
//! nobody listens on. A second launch now brings the first forward instead.
//! A devpit with a home of its own (`DEVPIT_HOME`, which the e2e suite uses)
//! is another devpit, and is left alone.

use devpit_core::{preference, Store};
use devpit_rpc::RpcError;
use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt as _, Shortcut, ShortcutState};

/// Whether this launch keeps to one copy: a devpit in its own home is
/// another devpit, and two of those may well be running on purpose.
pub(crate) fn one_copy() -> bool {
    std::env::var_os("DEVPIT_HOME").is_none_or(|home| home.is_empty())
}

/// devpit to the front, wherever it was.
pub(crate) fn forward(app: &AppHandle) {
    let _ = crate::island::raised(app);
}

/// The Dock icon clicked while the window is in the tray brings it back, as
/// every other way of opening devpit does.
pub(crate) fn reopened(app: &AppHandle, event: &tauri::RunEvent) {
    #[cfg(target_os = "macos")]
    if let tauri::RunEvent::Reopen { .. } = event {
        forward(app);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (app, event);
}

/// The window whose going ends devpit. The terminals live in tmux and outlive
/// it; a process kept for the island and the tray has nothing left to raise.
pub(crate) fn ends_devpit(label: &str) -> bool {
    label == "main"
}

/// What Quit in the tray does.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Quit {
    /// As the window's own close button: whatever closing asks, it asks here
    /// too, and the window going is what ends devpit.
    CloseTheWindow,
    /// No window left to ask through, so nothing stands in the way.
    Exit,
}

pub(crate) fn quitting(has_window: bool) -> Quit {
    if has_window {
        Quit::CloseTheWindow
    } else {
        Quit::Exit
    }
}

/// What closing the window does.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Close {
    /// Into the tray: hooks, notices, the agent door and the Remote go on.
    Hide,
    /// devpit ends; the terminals stay in tmux.
    Quit,
}

/// Hidden only when the person wants it, into a tray they can see, and never
/// on the way out: a window hidden with no icon to bring it back is a devpit
/// that looks gone and is not.
pub(crate) fn closing(keep: bool, tray_shown: bool, quitting: bool) -> Close {
    if keep && tray_shown && !quitting {
        Close::Hide
    } else {
        Close::Quit
    }
}

/// Whether `keys` is a shortcut the plugin can register.
pub(crate) fn readable(keys: &str) -> bool {
    keys.parse::<Shortcut>().is_ok()
}

/// Registers the chosen shortcut, in place of any before it.
pub(crate) fn shortcut_apply(app: &AppHandle, keys: Option<&str>) -> Result<(), String> {
    let shortcuts = app.global_shortcut();
    shortcuts.unregister_all().map_err(|err| err.to_string())?;
    let Some(keys) = keys.filter(|keys| !keys.is_empty()) else {
        return Ok(());
    };
    shortcuts
        .on_shortcut(keys, |app, _, event| {
            if event.state == ShortcutState::Pressed {
                forward(app);
            }
        })
        .map_err(|err| format!("{keys} could not be taken: {err}"))
}

/// The shortcut kept from before, registered as the app starts.
pub(crate) fn shortcut_restore(app: &AppHandle) {
    let kept = Store::open_default()
        .ok()
        .and_then(|store| store.preference(preference::SHORTCUT).ok().flatten());
    if let Some(keys) = kept {
        if let Err(why) = shortcut_apply(app, Some(&keys)) {
            eprintln!("devpit: the shortcut was not registered: {why}");
        }
    }
}

/// `shortcut.read` — the keys that bring devpit forward, if any.
#[tauri::command]
#[specta::specta]
pub async fn shortcut_read() -> Result<Option<String>, RpcError> {
    crate::off_main::blocking(|| Ok(Store::open_default()?.preference(preference::SHORTCUT)?)).await
}

/// `shortcut.set` — the keys that bring devpit forward, or none.
#[tauri::command]
#[specta::specta]
pub async fn shortcut_set(
    app: AppHandle,
    keys: Option<String>,
) -> Result<Option<String>, RpcError> {
    crate::off_main::blocking(move || {
        let keys = keys
            .map(|keys| keys.trim().to_owned())
            .filter(|keys| !keys.is_empty());
        if keys.as_deref().is_some_and(|keys| !readable(keys)) {
            return Err(RpcError::new(
                devpit_rpc::ErrorCode::Invalid,
                "that is not a shortcut devpit can take",
            ));
        }
        shortcut_apply(&app, keys.as_deref())
            .map_err(|why| RpcError::new(devpit_rpc::ErrorCode::Conflict, why))?;
        Store::open_default()?
            .set_preference(preference::SHORTCUT, keys.as_deref().unwrap_or(""))?;
        Ok(keys)
    })
    .await
}

/// `at_login.read` — whether devpit opens when the person logs in.
#[tauri::command]
#[specta::specta]
pub async fn at_login_read(app: AppHandle) -> Result<bool, RpcError> {
    crate::off_main::blocking(move || {
        app.autolaunch()
            .is_enabled()
            .map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}

/// `at_login.set` — opens devpit at login, or stops.
#[tauri::command]
#[specta::specta]
pub async fn at_login_set(app: AppHandle, on: bool) -> Result<bool, RpcError> {
    crate::off_main::blocking(move || {
        let launch = app.autolaunch();
        let done = if on {
            launch.enable()
        } else {
            launch.disable()
        };
        done.map_err(|err| RpcError::internal(err.to_string()))?;
        launch
            .is_enabled()
            .map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}

#[cfg(test)]
#[path = "desk_tests.rs"]
mod tests;
