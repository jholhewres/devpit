//! Decisions in Settings: the provider, its model and address, the day's cap,
//! the key, and the projects whose state is never sent.
//!
//! Off until a key is kept. The key lives in a private file beside the store,
//! never in SQLite, and is never read back to the window.

use std::path::PathBuf;

use devpit_core::{preference, Store};
use devpit_rpc::{Deciding, DecisionTried, ErrorCode, RpcError};
use serde_json::{json, Value};

use crate::deciding::{gate, now, today, Conf};
use crate::rubric::{Rubric, Threshold};

const OPENROUTER: &str = "https://openrouter.ai/api/v1";
const OPENROUTER_MODEL: &str = "typesafe/jev-1.13";
/// The gateway names the model its own way; the request is the same shape.
const VERCEL: &str = "https://ai-gateway.vercel.sh/v1";
const VERCEL_MODEL: &str = "typesafe-ai/jev";
const DEFAULT_CAP_USD: f64 = 1.0;
/// Past this a cap is a typo, not a budget.
const MOST_CAP_USD: f64 = 1000.0;

fn key_file() -> Result<PathBuf, RpcError> {
    Ok(Store::root()?.join("decisions-key"))
}

/// The kept key, for the requests and for `kept_out`.
pub(crate) fn key() -> Option<String> {
    let kept = std::fs::read_to_string(key_file().ok()?).ok()?;
    Some(kept.trim().to_owned()).filter(|key| !key.is_empty())
}

fn read_list(store: &Store, name: &str) -> Vec<String> {
    store
        .preference(name)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn read_settings(store: &Store) -> Result<Deciding, RpcError> {
    let read =
        |key: &str| -> Result<String, RpcError> { Ok(store.preference(key)?.unwrap_or_default()) };
    let provider = read(preference::DECISIONS_PROVIDER)?;
    let since = today(now());
    let (spent, decided) = store.decision_spend_since(since)?;
    Ok(Deciding {
        provider: if provider == "vercel" {
            provider
        } else {
            "openrouter".to_owned()
        },
        model: read(preference::DECISIONS_MODEL)?,
        url: read(preference::DECISIONS_URL)?,
        daily_cap_usd: read(preference::DECISIONS_DAILY_CAP)?
            .parse()
            .unwrap_or(DEFAULT_CAP_USD),
        key_set: key().is_some(),
        spent_today_usd: spent,
        decided_today: decided,
        opted_out: read_list(store, preference::DECISIONS_OPTED_OUT),
    })
}

/// What a decision needs, from the settings and the key file.
pub(crate) fn conf(store: &Store) -> Result<Conf, String> {
    let settings = read_settings(store).map_err(|err| err.message)?;
    let vercel = settings.provider == "vercel";
    let or_default = |set: &str, default: &str| {
        if set.is_empty() {
            default.to_owned()
        } else {
            set.to_owned()
        }
    };
    Ok(Conf {
        base: or_default(&settings.url, if vercel { VERCEL } else { OPENROUTER }),
        model: or_default(
            &settings.model,
            if vercel {
                VERCEL_MODEL
            } else {
                OPENROUTER_MODEL
            },
        ),
        cap_usd: settings.daily_cap_usd,
        key: key(),
        opted_out: settings.opted_out,
        modes: store
            .preference(preference::DECISIONS_MODES)
            .ok()
            .flatten()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or(Value::Null),
        secrets: crate::kept_out::what_devpit_gave(store),
    })
}

/// `decisions.read` — how Decisions is set up, and what it spent today.
#[tauri::command]
#[specta::specta]
pub async fn decisions_read() -> Result<Deciding, RpcError> {
    crate::off_main::blocking(|| read_settings(&Store::open_default()?)).await
}

/// `decisions.set` — the provider, the model, the address and the day's cap.
#[tauri::command]
#[specta::specta]
pub async fn decisions_set(
    provider: String,
    model: String,
    url: String,
    daily_cap_usd: f64,
) -> Result<Deciding, RpcError> {
    crate::off_main::blocking(move || {
        if !["openrouter", "vercel"].contains(&provider.as_str()) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "the provider is openrouter or vercel",
            ));
        }
        let url = url.trim().trim_end_matches('/').to_owned();
        if !reachable_address(&url) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "the address is https, or a service on this machine",
            ));
        }
        if !daily_cap_usd.is_finite() || !(0.0..=MOST_CAP_USD).contains(&daily_cap_usd) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "the daily cap is between 0 and 1000 dollars",
            ));
        }
        let model = model.trim();
        if model.len() > 200 || model.chars().any(char::is_whitespace) {
            return Err(RpcError::new(ErrorCode::Invalid, "that is not a model"));
        }
        let store = Store::open_default()?;
        store.set_preference(preference::DECISIONS_PROVIDER, &provider)?;
        store.set_preference(preference::DECISIONS_MODEL, model)?;
        store.set_preference(preference::DECISIONS_URL, &url)?;
        store.set_preference(preference::DECISIONS_DAILY_CAP, &daily_cap_usd.to_string())?;
        read_settings(&store)
    })
    .await
}

/// An address a key may be sent to: https, or this machine.
pub(crate) fn reachable_address(url: &str) -> bool {
    url.is_empty()
        || url.starts_with("https://")
        || url.starts_with("http://127.0.0.1")
        || url.starts_with("http://localhost")
}

/// `decisions.key_set` — keeps the provider's key in a private file, or
/// forgets it, which turns Decisions off.
#[tauri::command]
#[specta::specta]
pub async fn decisions_key_set(key: Option<String>) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let file = key_file()?;
        match key
            .map(|key| key.trim().to_owned())
            .filter(|key| !key.is_empty())
        {
            Some(key) if key.len() > 512 || key.chars().any(char::is_whitespace) => {
                Err(RpcError::new(ErrorCode::Invalid, "that is not a key"))
            }
            Some(key) => Ok(devpit_core::home::write_private(&file, key.as_bytes())
                .map_err(|err| RpcError::internal(err.to_string()))?),
            None => match std::fs::remove_file(&file) {
                Err(err) if err.kind() != std::io::ErrorKind::NotFound => {
                    Err(RpcError::internal(err.to_string()))
                }
                _ => Ok(()),
            },
        }
    })
    .await
}

/// `decisions.project_set` — whether a project's state may be sent at all.
#[tauri::command]
#[specta::specta]
pub async fn decisions_project_set(project_id: String, send: bool) -> Result<Deciding, RpcError> {
    crate::off_main::blocking(move || {
        let store = Store::open_default()?;
        let mut out = read_list(&store, preference::DECISIONS_OPTED_OUT);
        out.retain(|id| id != &project_id);
        if !send {
            out.push(project_id);
        }
        store.set_preference(preference::DECISIONS_OPTED_OUT, &json!(out).to_string())?;
        read_settings(&store)
    })
    .await
}

/// A question with an obvious answer, so a test says whether the call works
/// rather than what the model thinks.
fn trial() -> Rubric {
    Rubric {
        name: "settings.test".to_owned(),
        version: Some("1".to_owned()),
        questions: json!({ "blue": { "type": "noul", "instructions": "Does the state say the sky is blue?" } }),
        thresholds: [(
            "blue".to_owned(),
            Threshold {
                min: Some(0.5),
                ..Threshold::default()
            },
        )]
        .into(),
    }
}

/// `decisions.test` — one trivial question, through the same path every
/// decision takes: its latency and cost, or why it did not answer.
#[tauri::command]
#[specta::specta]
pub async fn decisions_test() -> Result<DecisionTried, RpcError> {
    crate::off_main::blocking(|| {
        let rubric = trial();
        let gated = gate(None, None, &rubric.name, &rubric, json!("The sky is blue."));
        Ok(match gated.skipped {
            Some(skip) => DecisionTried {
                probability: None,
                latency_ms: None,
                cost_usd: None,
                note: Some(skip.reason()),
            },
            None => DecisionTried {
                probability: gated.verdicts["blue"]["value"].as_f64(),
                latency_ms: Some(gated.latency_ms.min(u32::MAX as u64) as u32),
                cost_usd: Some(gated.cost_usd),
                note: None,
            },
        })
    })
    .await
}

#[cfg(test)]
#[path = "decisions_tests.rs"]
mod tests;
