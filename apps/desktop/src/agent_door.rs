//! Whether devpit's own MCP still answers, and starting it again when not.
//!
//! The MCP tools post to the hook listener's `/agent`. When that stopped
//! answering, only restarting the app brought it back, and that took the
//! orchestrator's chat with it. This checks the door the way a tool reaches
//! it, and opens a new one without touching the window or the terminals.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use devpit_rpc::{AgentHealth, RpcError};
use tauri::{AppHandle, Emitter};

/// How often the door is knocked on.
const EVERY: Duration = Duration::from_secs(30);
/// Checks in a row unanswered before it is opened again by itself.
pub(crate) const FAILURES_BEFORE_RESTART: u32 = 3;
/// The event the window hears a check or a restart on.
const CHECKED: &str = "agent:health";

static HEALTH: Mutex<AgentHealth> = Mutex::new(AgentHealth {
    answering: true,
    latency_ms: None,
    checked_at: None,
    failures: 0,
    restarts: 0,
    last_restart_at: None,
    detail: None,
});

/// Writes one check into `health`, and says whether it is time to restart.
pub(crate) fn heard(health: &mut AgentHealth, outcome: Result<Duration, String>, now: f64) -> bool {
    health.checked_at = Some(now);
    match outcome {
        Ok(took) => {
            health.answering = true;
            health.latency_ms = Some(took.as_secs_f64() * 1000.0);
            health.failures = 0;
            health.detail = None;
            false
        }
        Err(why) => {
            health.answering = false;
            health.latency_ms = None;
            health.failures += 1;
            health.detail = Some(why);
            health.failures >= FAILURES_BEFORE_RESTART
        }
    }
}

/// Writes a restart into `health`.
pub(crate) fn restarted(health: &mut AgentHealth, why: &str, now: f64) {
    health.restarts += 1;
    health.last_restart_at = Some(now);
    health.failures = 0;
    health.detail = Some(why.to_owned());
}

/// One question, asked the way a tool asks it: over loopback, through the
/// files the listener published.
fn knock() -> Result<Duration, String> {
    let root = devpit_core::Store::root().map_err(|err| err.to_string())?;
    let started = Instant::now();
    devpit_agentapi::client::ask(&root, "methods", serde_json::json!({}), &root)?;
    Ok(started.elapsed())
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|at| at.as_millis() as f64)
        .unwrap_or(0.0)
}

fn current() -> AgentHealth {
    HEALTH
        .lock()
        .map(|health| health.clone())
        .unwrap_or_else(|held| held.into_inner().clone())
}

/// Checks now, restarts when it has failed too often in a row, and tells the window.
fn check(app: &AppHandle) -> AgentHealth {
    let outcome = knock();
    let due = HEALTH
        .lock()
        .map(|mut health| heard(&mut health, outcome, now_ms()))
        .unwrap_or(false);
    if due {
        let why = format!("it stopped answering for {FAILURES_BEFORE_RESTART} checks in a row");
        if restart(app, &why).is_ok() {
            devpit_core::reports::background("devpit MCP", &format!("restarted by itself: {why}"));
            crate::channels::emit(crate::channels::Told {
                event: devpit_rpc::ChannelEvent::McpRestarted,
                line: "devpit's MCP stopped answering and was restarted.".to_owned(),
                bare: "devpit's MCP was restarted.".to_owned(),
                actions: Vec::new(),
            });
            crate::notices::ring(
                app,
                None,
                crate::notices::kind::AGENT,
                "devpit's MCP was restarted",
                Some("Its tools stopped answering, so devpit opened them again. Nothing else was restarted."),
                None,
            );
        }
    }
    let health = current();
    let _ = app.emit(CHECKED, health.clone());
    health
}

/// Opens a new door and retires the old one.
fn restart(app: &AppHandle, why: &str) -> Result<(), String> {
    let root = devpit_core::Store::root().map_err(|err| err.to_string())?;
    crate::listener::open(app.clone(), &root)?;
    if let Ok(mut health) = HEALTH.lock() {
        restarted(&mut health, why, now_ms());
    }
    Ok(())
}

/// Knocks every [`EVERY`] for as long as the app runs.
pub fn watch(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(EVERY);
        check(&app);
    });
}

/// The health as last checked, after checking again.
pub(crate) fn health_now(app: &AppHandle) -> AgentHealth {
    check(app)
}

/// `agent.health` — whether devpit's own MCP answers, checked now.
#[tauri::command]
#[specta::specta]
pub async fn agent_health(app: AppHandle) -> Result<AgentHealth, RpcError> {
    crate::off_main::blocking(move || Ok(check(&app))).await
}

/// A restart the person asked for, from the window or the Remote.
pub(crate) fn restart_by_hand(app: &AppHandle) -> Result<AgentHealth, String> {
    restart(app, "restarted by hand")?;
    Ok(check(app))
}

/// `agent.restart` — opens devpit's own MCP again, without restarting the app.
#[tauri::command]
#[specta::specta]
pub async fn agent_restart(app: AppHandle) -> Result<AgentHealth, RpcError> {
    crate::off_main::blocking(move || restart_by_hand(&app).map_err(RpcError::internal)).await
}

#[cfg(test)]
#[path = "agent_door_tests.rs"]
mod tests;
