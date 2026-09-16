//! Contract commands, adapted to Tauri's transport.
//!
//! Thin on purpose: this layer translates, it does not decide. Every question
//! with a product answer is answered by `crates/`, which is why the same
//! contract will serve an HTTP server and a CLI without a rewrite.

use devpit_core::Store;
use devpit_rpc::{AppInfo, RpcError};

/// `app.info` — version, platform, and where state lives.
#[tauri::command]
#[specta::specta]
pub fn app_info() -> Result<AppInfo, RpcError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_owned(),
        platform: std::env::consts::OS.to_owned(),
        state_path: Store::default_path()?.display().to_string(),
    })
}
