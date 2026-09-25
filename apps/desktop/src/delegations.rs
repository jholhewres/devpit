//! What an orchestrator's conversations passed to other sessions and heard
//! back, read from the CLI's own logs of its folder: the history the Agents
//! panel shows, and who woke a turn nobody here asked for.

use std::collections::BTreeMap;
use std::path::Path;

use devpit_agentcli::peers::{events, logs_of, newest, tail, woken_by, PeerEvent};
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// How many of the folder's logs are read, newest first.
const LOGS: usize = 30;
/// How many exchanges a session keeps in the panel.
const KEPT: usize = 40;
/// How much of a message is quoted at the top of a woken turn.
const QUOTED: usize = 900;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    /// `sent`, `heard` or `idle`.
    pub kind: String,
    /// When, as the CLI wrote it (RFC 3339).
    pub at: String,
    /// The request's summary, for one that was sent.
    pub summary: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AgentThread {
    /// The session, by the name it is messaged by.
    pub name: String,
    pub last_at: String,
    /// Oldest first.
    pub events: Vec<AgentEvent>,
}

/// `orchestrator.agents` — every session an orchestrator has written to or
/// heard from, with what passed, most recent first.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_agents(project_id: String) -> Result<Vec<AgentThread>, RpcError> {
    crate::off_main::blocking(move || {
        let projects = crate::projects::project_list_now()?.projects;
        let here = projects
            .iter()
            .find(|one| one.id == project_id)
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such project"))?;
        let profile = here
            .orchestrator
            .as_deref()
            .ok_or_else(|| RpcError::new(ErrorCode::Invalid, "only an orchestrator has agents"))?;
        let config = crate::live_sessions::config_of(profile)?;
        let root = Path::new(&here.root_path);
        let logs = logs_of(
            &config,
            &root.canonicalize().unwrap_or_else(|_| root.to_path_buf()),
        );
        Ok(threads(&logs))
    })
    .await
}

pub(crate) fn threads(logs: &Path) -> Vec<AgentThread> {
    let mut files: Vec<_> = std::fs::read_dir(logs)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "jsonl"))
                .filter_map(|entry| Some((entry.metadata().ok()?.modified().ok()?, entry.path())))
                .collect()
        })
        .unwrap_or_default();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    let mut by_name: BTreeMap<String, Vec<AgentEvent>> = BTreeMap::new();
    for (_, file) in files.into_iter().take(LOGS) {
        for event in events(&tail(&file)) {
            let name = event.peer().to_owned();
            if name.is_empty() {
                continue;
            }
            by_name.entry(name).or_default().push(shown(event));
        }
    }
    let mut out: Vec<AgentThread> = by_name
        .into_iter()
        .map(|(name, mut events)| {
            events.sort_by(|a, b| a.at.cmp(&b.at));
            let over = events.len().saturating_sub(KEPT);
            events.drain(..over);
            AgentThread {
                last_at: events.last().map(|one| one.at.clone()).unwrap_or_default(),
                name,
                events,
            }
        })
        .collect();
    out.sort_by(|a, b| b.last_at.cmp(&a.last_at));
    out
}

fn shown(event: PeerEvent) -> AgentEvent {
    match event {
        PeerEvent::Sent {
            at,
            summary,
            message,
            ..
        } => AgentEvent {
            kind: "sent".into(),
            at,
            summary: Some(summary).filter(|said| !said.is_empty()),
            text: message,
        },
        PeerEvent::Heard { at, body, .. } => AgentEvent {
            kind: "heard".into(),
            at,
            summary: None,
            text: body,
        },
        PeerEvent::Idle { at, said, .. } => AgentEvent {
            kind: "idle".into(),
            at,
            summary: None,
            text: said,
        },
    }
}

/// Said at the top of a turn nobody here asked for: who woke it, and what they
/// said, when the log says; otherwise every way it could have been.
pub(crate) fn woken_label(logs: &Path) -> String {
    let said = newest(logs).and_then(|log| woken_by(&tail(&log)));
    match said {
        Some(PeerEvent::Heard { from, body, .. }) => {
            let mut quoted: String = body.chars().take(QUOTED).collect();
            if body.chars().count() > QUOTED {
                quoted.push('…');
            }
            let quoted = quoted
                .lines()
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("↪ *`{from}` wrote:*\n\n{quoted}\n\n")
        }
        Some(PeerEvent::Idle { from, .. }) => format!("↪ *`{from}` finished a turn.*\n\n"),
        _ => "↪ *Not from this chat — written from your phone or claude.ai, another session wrote, or work it left running finished.*\n\n".to_owned(),
    }
}

#[cfg(test)]
#[path = "delegations_tests.rs"]
mod tests;
