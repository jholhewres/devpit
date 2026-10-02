//! What a desktop app is expected to have: one copy running, a place in the
//! tray, opening at login, and a shortcut that brings it forward from
//! anywhere.
//!
//! One copy, because a second one is not harmless: it opens its own hook
//! listener and its own agent door, writes their endpoint and secret over the
//! first one's, and every session's hooks and MCP calls then post to a port
//! nobody listens on. A second launch now brings the first forward instead.
//! A devpit with a home of its own (`DEVPIT_HOME`, which the e2e suite uses)
//! is another devpit, and is left alone.

use devpit_core::{preference, Store};
use devpit_rpc::RpcError;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt as _, Shortcut, ShortcutState};

const TRAY: &str = "devpit";

/// Whether this launch keeps to one copy: a devpit in its own home is
/// another devpit, and two of those may well be running on purpose.
pub(crate) fn one_copy() -> bool {
    std::env::var_os("DEVPIT_HOME").is_none_or(|home| home.is_empty())
}

/// devpit to the front, wherever it was.
pub(crate) fn forward(app: &AppHandle) {
    let _ = crate::island::raised(app);
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let open = MenuItem::with_id(app, "open", "Open devpit", true, None::<&str>)?;
    let pause = if crate::pausing::paused() {
        MenuItem::with_id(app, "resume", "Resume", true, None::<&str>)?
    } else {
        MenuItem::with_id(app, "pause", "Pause for an hour", true, None::<&str>)?
    };
    let quit = MenuItem::with_id(app, "quit", "Quit devpit", true, None::<&str>)?;
    Menu::with_items(
        app,
        &[&open, &pause, &PredefinedMenuItem::separator(app)?, &quit],
    )
}

/// Whether this desktop can show a tray icon. On Linux the icon goes through
/// the appindicator library, loaded when first used — and its loader panics
/// when the library is not there, which would take the whole app with it.
#[cfg(target_os = "linux")]
fn can_tray() -> bool {
    const NAMES: [&str; 2] = ["libayatana-appindicator3.so.1", "libappindicator3.so.1"];
    let mut dirs: Vec<std::path::PathBuf> = [
        "/usr/lib",
        "/usr/lib64",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
        "/lib/x86_64-linux-gnu",
        "/lib/aarch64-linux-gnu",
    ]
    .iter()
    .map(std::path::PathBuf::from)
    .collect();
    // An AppImage carries its own.
    if let Some(appdir) = std::env::var_os("APPDIR") {
        let appdir = std::path::PathBuf::from(appdir);
        dirs.push(appdir.join("usr/lib"));
        dirs.push(appdir.join("usr/lib/x86_64-linux-gnu"));
        dirs.push(appdir.join("usr/lib/aarch64-linux-gnu"));
    }
    dirs.iter()
        .any(|dir| NAMES.iter().any(|name| dir.join(name).exists()))
}

#[cfg(not(target_os = "linux"))]
fn can_tray() -> bool {
    true
}

/// The tray icon and its menu. A desktop without a tray just has none.
pub(crate) fn tray(app: &AppHandle) {
    if !can_tray() {
        return;
    }
    // A last guard: the library found, and its loader panicking all the same.
    let app = app.clone();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || build_tray(&app)));
}

fn build_tray(app: &AppHandle) {
    let Ok(menu) = menu(app) else {
        return;
    };
    let mut built = TrayIconBuilder::with_id(TRAY)
        .tooltip("devpit")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => forward(app),
            "pause" => crate::pausing::for_an_hour(app),
            "resume" => crate::pausing::resume(app),
            // As the window's own close button: whatever closing asks, it
            // asks here too.
            "quit" => {
                if let Some(main) = app.get_webview_window("main") {
                    let _ = main.close();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                forward(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon().cloned() {
        built = built.icon(icon);
    }
    let _ = built.build(app);
}

/// The tray's menu again, after the pause changed.
pub(crate) fn tray_refresh(app: &AppHandle) {
    if let (Some(tray), Ok(menu)) = (app.tray_by_id(TRAY), menu(app)) {
        let _ = tray.set_menu(Some(menu));
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
