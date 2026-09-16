//! Contract commands, adapted to Tauri's transport.
//!
//! Thin on purpose: this layer translates, it does not decide. Every question
//! with a product answer is answered by `crates/`, which is why the same
//! contract will serve an HTTP server and a CLI without a rewrite.

use devpit_core::Store;
use devpit_rpc::{AppInfo, RpcError};

/// `app.info` — version, platform, and where state lives.
///
/// The version is the one the updater compares against, not the crate's: the
/// two agree in a release, and when a build sets them apart the screen should
/// say what an update will be measured from.
#[tauri::command]
#[specta::specta]
pub fn app_info(app: tauri::AppHandle) -> Result<AppInfo, RpcError> {
    Ok(AppInfo {
        version: app.package_info().version.to_string(),
        platform: std::env::consts::OS.to_owned(),
        state_path: Store::default_path()?.display().to_string(),
    })
}
