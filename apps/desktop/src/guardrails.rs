//! The confirmations an orchestrator needs from the person, each on a mode
//! the person picks per orchestrator: following its chat's permission (no
//! confirmation while the chat runs in Full access), always, or never.
//!
//! Kept among the orchestrator's settings, out of its chat's reach. The chat's
//! permission is read where the chat keeps it, its conversation's head.

use std::path::Path;

use devpit_agentcli::head::{head_path, read_head};
use devpit_rpc::{Guardrail, GuardrailMode, GuardrailRow, Guardrails, Project, RpcError};
use serde_json::{json, Value};

/// Every guardrail, in the order Settings lists them.
const ALL: [Guardrail; 3] = [Guardrail::Stop, Guardrail::ForceStop, Guardrail::SendDraft];
/// Their key among the orchestrator's settings.
const KEY: &str = "guardrails";
/// The CLI's own word for Full access.
const FULL_ACCESS: &str = "bypassPermissions";

fn name(guardrail: Guardrail) -> String {
    serde_json::to_value(guardrail)
        .ok()
        .and_then(|named| named.as_str().map(str::to_owned))
        .unwrap_or_default()
}

/// What the person reads, in the log.
fn label(guardrail: Guardrail) -> &'static str {
    match guardrail {
        Guardrail::Stop => "Stop a session",
        Guardrail::ForceStop => "Force stop, or a stop that loses work",
        Guardrail::SendDraft => "Send a drafted reply as you",
    }
}

fn mode_label(mode: GuardrailMode) -> &'static str {
    match mode {
        GuardrailMode::Follow => "Follow chat permission",
        GuardrailMode::Always => "Always ask",
        GuardrailMode::Never => "Never ask",
    }
}

/// Untouched, a guardrail asks as it always did outside Full access; losing
/// work asks even there.
fn mode(settings: &Value, guardrail: Guardrail) -> GuardrailMode {
    serde_json::from_value(settings[KEY][name(guardrail)].clone()).unwrap_or(match guardrail {
        Guardrail::ForceStop => GuardrailMode::Always,
        Guardrail::Stop | Guardrail::SendDraft => GuardrailMode::Follow,
    })
}

pub(crate) fn rows(settings: &Value) -> Guardrails {
    Guardrails {
        rows: ALL
            .into_iter()
            .map(|guardrail| GuardrailRow {
                guardrail,
                mode: mode(settings, guardrail),
            })
            .collect(),
    }
}

pub(crate) fn set(settings: &mut Value, guardrail: Guardrail, mode: GuardrailMode) {
    if !settings[KEY].is_object() {
        settings[KEY] = json!({});
    }
    settings[KEY][name(guardrail)] = json!(mode);
}

/// Whether the orchestrator's chat runs in Full access: its newest
/// conversation in `sessions`, the one speaking now.
pub(crate) fn full_access(sessions: &Path) -> bool {
    devpit_agentcli::history::conversations(sessions)
        .first()
        .and_then(|newest| read_head(&head_path(sessions, &newest.id)))
        .and_then(|head| head.permission)
        .as_deref()
        == Some(FULL_ACCESS)
}

/// Whether `guardrail` waits for the person, with the orchestrator's chat
/// kept in `sessions`.
pub(crate) fn waits(settings: &Value, sessions: &Path, guardrail: Guardrail) -> bool {
    match mode(settings, guardrail) {
        GuardrailMode::Always => true,
        GuardrailMode::Never => false,
        GuardrailMode::Follow => !full_access(sessions),
    }
}

/// Whether `guardrail` lets the orchestrator `here` act at its own call.
/// Anything unread asks, as before.
pub(crate) fn off(here: &Project, guardrail: Guardrail) -> bool {
    let Ok(store) = crate::projects::store() else {
        return false;
    };
    let Ok(settings) = store.orchestrator_settings(Path::new(&here.root_path)) else {
        return false;
    };
    crate::projects::project_home(&here.id)
        .is_ok_and(|home| !waits(&settings, &home.sessions(), guardrail))
}

/// `answer`, for something `did` at the orchestrator's call because
/// `guardrail` did not ask: the line the chat shows, kept in the sessions log
/// of the orchestrator in `folder` too.
pub(crate) fn reported(folder: &Path, guardrail: Guardrail, did: &str, mut answer: Value) -> Value {
    let line = format!(
        "{did}, at the orchestrator's call without asking: «{}» does not ask now.",
        label(guardrail)
    );
    let _ = crate::orchestrator_notes::note(
        folder,
        "sessions",
        &line,
        crate::orchestrator_notes::Section::Log,
    );
    answer["unasked"] = json!(line);
    answer["note"] = json!("Done without asking the person, as their Settings allow. Tell them in one line what you did, why, and what it lost.");
    answer
}

/// `orchestrator.guardrails` — an orchestrator's guardrails and their modes.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_guardrails(project_id: String) -> Result<Guardrails, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let folder = crate::assistant::folder_of(&store, &project_id)?;
        Ok(rows(&store.orchestrator_settings(&folder)?))
    })
    .await
}

/// `orchestrator.guardrail_set` — one guardrail's mode, as the person chose
/// it; the change is kept in the orchestrator's sessions log.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_guardrail_set(
    project_id: String,
    guardrail: Guardrail,
    mode: GuardrailMode,
) -> Result<Guardrails, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let folder = crate::assistant::folder_of(&store, &project_id)?;
        let mut settings = store.orchestrator_settings(&folder)?;
        let before = self::mode(&settings, guardrail);
        set(&mut settings, guardrail, mode);
        store.set_orchestrator_setting(&folder, KEY, settings[KEY].clone())?;
        if before != mode {
            let _ = crate::orchestrator_notes::note(
                &folder,
                "sessions",
                &format!(
                    "Guardrail «{}» set to {} in Settings (was {}).",
                    label(guardrail),
                    mode_label(mode),
                    mode_label(before)
                ),
                crate::orchestrator_notes::Section::Log,
            );
        }
        Ok(rows(&settings))
    })
    .await
}

#[cfg(test)]
#[path = "guardrails_tests.rs"]
mod tests;
