//! How much of an Anthropic plan's quota windows is used, and when each resets.
//!
//! The one network call on the usage screen, and it spends no tokens: the same
//! endpoint the CLI's own `/usage` reads, with the installation's saved sign-in.
//! Asked at most every five minutes per installation. The token is read,
//! sent, and never logged or put in an error.
//!
//! It reads the CLI's own credentials, calls an endpoint Anthropic does not
//! document, and identifies itself as the CLI while doing it. That is a
//! decision taken with those three things in view, not an oversight: the
//! number is why the page exists, and it is read on the same machine, with the
//! sign-in the person already gave the same account. If the endpoint changes
//! or Anthropic says no, this reader goes and the rest of the page — what the
//! transcripts say was spent — stands on its own.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use devpit_agentcli::spend_scan::epoch_of;
use devpit_rpc::{PlanLimits, PlanWindow, RpcError};
use serde_json::Value;

const USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
/// The endpoint refuses a request without it.
const OAUTH_BETA: &str = "oauth-2025-04-20";
const USER_AGENT: &str = "claude-code/2.1.0";
const FRESH_FOR: Duration = Duration::from_secs(5 * 60);

/// The saved sign-in, as much of it as the request and the label need.
pub(crate) struct SignIn {
    pub token: String,
    pub subscription: Option<String>,
    pub tier: Option<String>,
}

/// The Claude sign-in in a `.credentials.json`, or why there is none.
pub(crate) fn sign_in_of(credentials: &str) -> Result<SignIn, &'static str> {
    let value: Value = serde_json::from_str(credentials.trim())
        .map_err(|_| "the saved sign-in could not be read")?;
    let oauth = value
        .get("claudeAiOauth")
        .ok_or("this installation is not signed in to a Claude account")?;
    let field = |name: &str| {
        oauth
            .get(name)
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    Ok(SignIn {
        token: field("accessToken")
            .ok_or("this installation is not signed in to a Claude account")?,
        subscription: field("subscriptionType"),
        tier: field("rateLimitTier"),
    })
}

/// The quota windows in the endpoint's answer: the `limits` array when there is
/// one, the older flat fields otherwise.
pub(crate) fn windows_of(body: &Value) -> Vec<PlanWindow> {
    let reset = |value: Option<&Value>| {
        value
            .and_then(Value::as_str)
            .and_then(epoch_of)
            .map(|at| at as f64)
    };
    if let Some(limits) = body.get("limits").and_then(Value::as_array) {
        let windows: Vec<PlanWindow> = limits
            .iter()
            .filter_map(|entry| {
                let label = match entry.get("kind")?.as_str()? {
                    "session" => "5-hour".to_owned(),
                    "weekly_all" => "Weekly".to_owned(),
                    "weekly_scoped" => format!(
                        "Weekly · {}",
                        entry
                            .pointer("/scope/model/display_name")
                            .and_then(Value::as_str)
                            .unwrap_or("model")
                    ),
                    _ => return None,
                };
                Some(PlanWindow {
                    label,
                    percent: entry.get("percent")?.as_f64()?.clamp(0.0, 100.0),
                    resets_at: reset(entry.get("resets_at")),
                })
            })
            .collect();
        if !windows.is_empty() {
            return windows;
        }
    }
    [
        ("five_hour", "5-hour"),
        ("seven_day", "Weekly"),
        ("seven_day_opus", "Weekly · Opus"),
        ("seven_day_sonnet", "Weekly · Sonnet"),
    ]
    .into_iter()
    .filter_map(|(field, label)| {
        let window = body.get(field)?;
        Some(PlanWindow {
            label: label.to_owned(),
            percent: window.get("utilization")?.as_f64()?.clamp(0.0, 100.0),
            resets_at: reset(window.get("resets_at")),
        })
    })
    .collect()
}

/// "Max (5x)" from a `max` subscription on a `default_claude_max_5x` tier.
pub(crate) fn plan_label(subscription: Option<&str>, tier: Option<&str>) -> Option<String> {
    let plan = subscription?.to_ascii_lowercase();
    let mut label = match plan.as_str() {
        "max" => "Max".to_owned(),
        "pro" => "Pro".to_owned(),
        "team" => "Team".to_owned(),
        "enterprise" => "Enterprise".to_owned(),
        other => other.to_owned(),
    };
    if let Some(multiple) = tier
        .unwrap_or_default()
        .to_ascii_lowercase()
        .split(['_', '-'])
        .find(|word| {
            word.len() > 1
                && word.ends_with('x')
                && word[..word.len() - 1].chars().all(|c| c.is_ascii_digit())
        })
    {
        label.push_str(&format!(" ({multiple})"));
    }
    Some(label)
}

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or_default()
}

fn said(installation: &str, problem: &str) -> PlanLimits {
    PlanLimits {
        installation: installation.to_owned(),
        plan: None,
        windows: Vec::new(),
        read_at: now(),
        problem: Some(problem.to_owned()),
    }
}

fn fresh() -> &'static Mutex<HashMap<String, (Instant, PlanLimits)>> {
    static FRESH: OnceLock<Mutex<HashMap<String, (Instant, PlanLimits)>>> = OnceLock::new();
    FRESH.get_or_init(Mutex::default)
}

/// `plan.limits` — the quota windows of an installation's plan.
#[tauri::command]
#[specta::specta]
pub async fn plan_limits(installation: Option<String>) -> Result<PlanLimits, RpcError> {
    let store = crate::projects::store()?;
    let chosen = crate::installations::chosen(installation.as_deref())?;
    let directory = chosen.directory.display().to_string();
    if !crate::spend_history::billed(&chosen, &crate::agent_profiles::declared(&store)) {
        return Ok(said(
            &directory,
            "this installation runs against another provider, which has no Claude plan limits",
        ));
    }
    drop(store);
    if let Some((_, known)) = fresh().lock().ok().and_then(|cache| {
        cache
            .get(&directory)
            .filter(|(at, _)| at.elapsed() < FRESH_FOR)
            .cloned()
    }) {
        return Ok(known);
    }

    let Ok(credentials) = std::fs::read_to_string(chosen.directory.join(".credentials.json"))
    else {
        return Ok(said(
            &directory,
            "this installation has no saved sign-in; open claude there to sign in",
        ));
    };
    let sign_in = match sign_in_of(&credentials) {
        Ok(sign_in) => sign_in,
        Err(why) => return Ok(said(&directory, why)),
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|err| RpcError::internal(err.to_string()))?;
    let answer = client
        .get(USAGE_URL)
        .bearer_auth(&sign_in.token)
        .header("anthropic-beta", OAUTH_BETA)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await;
    let response = match answer {
        Ok(response) => response,
        Err(_) => return Ok(said(&directory, "the plan's usage could not be reached")),
    };
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Ok(said(
            &directory,
            "the saved sign-in was refused; open claude there to sign in again",
        ));
    }
    if !status.is_success() {
        return Ok(said(
            &directory,
            &format!("the plan's usage answered {}", status.as_u16()),
        ));
    }
    let Ok(body) = response.json::<Value>().await else {
        return Ok(said(
            &directory,
            "the plan's usage answered with something unreadable",
        ));
    };
    let limits = PlanLimits {
        installation: directory.clone(),
        plan: plan_label(sign_in.subscription.as_deref(), sign_in.tier.as_deref()),
        windows: windows_of(&body),
        read_at: now(),
        problem: None,
    };
    if let Ok(mut cache) = fresh().lock() {
        cache.insert(directory, (Instant::now(), limits.clone()));
    }
    Ok(limits)
}

#[cfg(test)]
#[path = "plan_limits_tests.rs"]
mod tests;
