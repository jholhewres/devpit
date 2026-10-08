//! devpit Plus on this install: the entitlement devpit.app signs, kept on
//! disk and checked offline against the key built in.
//!
//! Without one, or past its `expires`, only what needs devpit.app stops —
//! relay, push, channels, sync; everything that runs here goes on. Until a
//! key is built in (`PUBLIC_KEY`), Plus is not checked and not drawn at all.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use devpit_rpc::{PlusStatus, RpcError};
use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
use serde::Deserialize;

/// devpit.app's entitlement key, base64. None until billing is switched on.
const PUBLIC_KEY: Option<&str> = None;

/// What an entitlement says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct Claims {
    pub v: u32,
    pub account: String,
    pub plan: String,
    pub expires: i64,
}

/// The claims of `token` signed by `public` (base64), if it holds at `now`.
pub(crate) fn verified(public: &str, token: &str, now: i64) -> Option<Claims> {
    let key: [u8; 32] = STANDARD.decode(public.trim()).ok()?.try_into().ok()?;
    let key = VerifyingKey::from_bytes(&key).ok()?;
    let (payload, signature) = token.trim().split_once('.')?;
    let payload = URL_SAFE_NO_PAD.decode(payload).ok()?;
    let signature: [u8; 64] = URL_SAFE_NO_PAD.decode(signature).ok()?.try_into().ok()?;
    key.verify(&payload, &Signature::from_bytes(&signature))
        .ok()?;
    let claims: Claims = serde_json::from_slice(&payload).ok()?;
    (claims.v == 1 && claims.expires > now).then_some(claims)
}

fn file() -> Option<std::path::PathBuf> {
    devpit_core::Store::root()
        .ok()
        .map(|root| root.join("plus-entitlement"))
}

/// Where Plus stands, read from the entitlement kept here.
pub(crate) fn status_with(public: Option<&str>, token: Option<&str>, now: i64) -> PlusStatus {
    let Some(public) = public else {
        return PlusStatus {
            enabled: false,
            active: false,
            plan: None,
            expires_at: None,
        };
    };
    let claims = token.and_then(|token| verified(public, token, now));
    PlusStatus {
        enabled: true,
        active: claims.is_some(),
        plan: claims.as_ref().map(|one| one.plan.clone()),
        expires_at: claims.map(|one| one.expires as f64),
    }
}

fn status_now() -> PlusStatus {
    let token = file().and_then(|path| std::fs::read_to_string(path).ok());
    status_with(
        PUBLIC_KEY,
        token.as_deref(),
        devpit_core::reports::now() as i64,
    )
}

/// `plus.status` — whether Plus holds on this install.
#[tauri::command]
#[specta::specta]
pub async fn plus_status() -> Result<PlusStatus, RpcError> {
    crate::off_main::blocking(|| Ok(status_now())).await
}

#[derive(Deserialize)]
struct Answer {
    token: String,
}

/// `plus.refresh` — asks devpit.app for the account's entitlement and keeps it.
#[tauri::command]
#[specta::specta]
pub async fn plus_refresh() -> Result<PlusStatus, RpcError> {
    if PUBLIC_KEY.is_none() {
        return Ok(status_now());
    }
    let Some(account) = crate::account::token::read() else {
        return Ok(status_now());
    };
    let answer = crate::account::client()?
        .get(format!("{}/v1/plus/entitlement", crate::account::origin()))
        .bearer_auth(&account)
        .send()
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?;
    let path = file().ok_or_else(|| RpcError::internal("no home directory to write to"))?;
    if answer.status() == reqwest::StatusCode::NOT_FOUND {
        // No active Plus: whatever was kept no longer stands.
        let _ = std::fs::remove_file(&path);
    } else if answer.status().is_success() {
        let kept: Answer = answer
            .json()
            .await
            .map_err(|err| RpcError::internal(err.to_string()))?;
        devpit_core::home::write_private(&path, kept.token.as_bytes())
            .map_err(|err| RpcError::internal(err.to_string()))?;
    }
    crate::off_main::blocking(|| Ok(status_now())).await
}

#[cfg(test)]
#[path = "plus_tests.rs"]
mod tests;
