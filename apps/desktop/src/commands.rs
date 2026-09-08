//! Contract commands, adapted to Tauri's transport.
//!
//! Thin on purpose: this layer translates, it does not decide. Every question
//! with a product answer is answered by `crates/`, which is why the same
//! contract will serve an HTTP server and a CLI without a rewrite.

use quockpit_core::Store;
use quockpit_rpc::{AppHealth, AppInfo, Capabilities, RpcError};

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

/// `app.health` — alive, and is it the build you think it is?
#[tauri::command]
#[specta::specta]
pub fn app_health() -> Result<AppHealth, RpcError> {
    Ok(AppHealth {
        ok: true,
        // Single process in this phase: the running binary is the binary.
        // When `apps/server` exists this is where the mtime comparison goes —
        // the field is already in the contract so the screen will not change.
        stale: false,
        state_path: Store::default_path()?.display().to_string(),
    })
}

/// `app.capabilities` — what this build can do.
#[tauri::command]
#[specta::specta]
pub fn app_capabilities() -> Capabilities {
    Capabilities {
        // All false, and honestly so: the screen hides the panel instead of
        // showing it broken. Each flips in the commit that implements it.
        memory: false,
        cloud: false,
        vault: false,
        tmux: quockpit_tmux::Server::available(),
    }
}
