//! An orchestrator's conversation, kept running between turns.
//!
//! Every other chat starts a process per turn and lets it go, which is right
//! for a conversation only its person speaks in. An orchestrator is also
//! spoken to by the sessions it follows — "done", "blocked", the notice it
//! asked for when one went idle — and those reach only a process that is
//! still there. So its process stays (`devpit_agentcli::resident`), the
//! person's turns are lines on its stdin, and a turn some other session wakes
//! is written into the conversation here, streamed through the relay, and
//! announced so an open chat joins it.
//!
//! Only outside the supervised mode: asking before a tool runs is held per
//! turn by the window that sent it, and a woken turn has no such window.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use devpit_agentcli::driver::Driver;
use devpit_agentcli::resident::{Heard, Resident};
use devpit_agentcli::talk::{say, Said, Say};
use devpit_agentcli::AgentError;
use devpit_rpc::{ErrorCode, Frame, Message, Part, Role, RpcError};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager};

use crate::chat_listening::{Ear, Woken};

/// The event a woken turn is announced on, with its conversation id.
pub(crate) const WOKE: &str = "chat:woke";

/// What stays for one conversation.
struct Live {
    resident: Resident,
    /// The flags it was started with: a turn asking for others restarts it.
    started_as: String,
    project_id: String,
    ear: Arc<Ear>,
}

#[derive(Default)]
pub(crate) struct Residents(Mutex<HashMap<String, Live>>);

/// What a turn needs to go through the conversation's resident process.
pub(crate) struct Staying {
    pub app: AppHandle,
    pub project_id: String,
    pub conversation_id: String,
    pub transcript: PathBuf,
    pub head: PathBuf,
    pub driver: String,
}

/// What a turn needs to stay, when it is to: an orchestrator's, or any chat
/// asked to be reachable by Remote Control — outside the supervised mode.
pub(crate) fn staying(
    app: &AppHandle,
    project_id: &str,
    conversation_id: &str,
    sessions: &std::path::Path,
    driver: &str,
    mode: Option<&str>,
) -> Option<Staying> {
    stays(app, project_id, conversation_id, mode).then(|| Staying {
        app: app.clone(),
        project_id: project_id.to_owned(),
        conversation_id: conversation_id.to_owned(),
        transcript: devpit_agentcli::store::conversation_path(sessions, conversation_id),
        head: devpit_agentcli::head::head_path(sessions, conversation_id),
        driver: driver.to_owned(),
    })
}

fn stays(app: &AppHandle, project_id: &str, conversation_id: &str, mode: Option<&str>) -> bool {
    if mode == Some(crate::asking::ASKING_MODE) {
        return false;
    }
    // Remote Control lives in the process, so a reachable chat keeps one.
    if crate::chat_remote::wanted(app, conversation_id) {
        return true;
    }
    let Ok(store) = crate::projects::store() else {
        return false;
    };
    let Ok((_, root)) = crate::projects::locate(&store, project_id) else {
        return false;
    };
    devpit_core::Store::root()
        .ok()
        .and_then(|home| home.canonicalize().ok())
        .and_then(|home| devpit_core::home::orchestrator_of(&store, &home, &root))
        .is_some()
}

/// What a turn [`or_say`] could not finish answers with. Every one of these
/// is the agent's process — missing, gone or failing — and not a devpit bug,
/// so none of them reaches the error report.
pub(crate) fn turn_refused(err: AgentError) -> RpcError {
    let code = match err {
        AgentError::NotInstalled => ErrorCode::NotFound,
        // Sending again starts a process of its own.
        AgentError::Failed { .. } | AgentError::Unreadable(_) => ErrorCode::Busy,
    };
    RpcError::new(code, err.to_string())
}

/// A turn, through the conversation's resident process when there is to be
/// one, and as a process of its own otherwise.
///
/// `on_began` runs once the turn has the process, before its prompt is sent;
/// a turn stopped while it waited for a woken one never gets there, and ends
/// `cancelled`.
pub(crate) fn or_say(
    staying: Option<Staying>,
    driver: &dyn Driver,
    turn: &Say<'_>,
    on_began: impl FnOnce(),
    mut on_part: impl FnMut(Part),
    mut on_start: impl FnMut(u32),
) -> Result<Said, AgentError> {
    let Some(staying) = staying else {
        on_began();
        return say(driver, turn, on_part, on_start);
    };
    let residents = staying.app.state::<Residents>();
    let (tell, heard) = mpsc::channel();
    let ear = {
        let mut all = residents.0.lock().map_err(|_| AgentError::NotInstalled)?;
        let wanted = started_as(turn);
        if all
            .get(&staying.conversation_id)
            .is_some_and(|live| live.started_as != wanted)
        {
            if let Some(old) = all.remove(&staying.conversation_id) {
                old.resident.close();
            }
        }
        let fresh = !all.contains_key(&staying.conversation_id);
        if fresh {
            let listening = all
                .iter()
                .map(|(key, live)| (key.as_str(), live.project_id.as_str()));
            for key in evicted(listening, &staying.project_id, |key| {
                crate::chat_remote::wanted(&staying.app, key)
            }) {
                if let Some(old) = all.remove(&key) {
                    old.resident.close();
                }
            }
            let live = start(&staying, turn, wanted)?;
            all.insert(staying.conversation_id.clone(), live);
        }
        let ear = all
            .get(&staying.conversation_id)
            .map(|live| live.ear.clone())
            .ok_or(AgentError::NotInstalled)?;
        drop(all);
        // A new process is a new connection: one asked to be reachable is
        // reached again.
        if fresh {
            crate::chat_remote::reconnect(&staying.app, &staying.conversation_id);
        }
        ear
    };
    // A woken turn still running finishes first: the person's words go in
    // after it, not into the middle of it.
    if !ear.claim(&tell) {
        return Ok(stopped());
    }
    on_began();
    {
        let mut all = residents.0.lock().map_err(|_| AgentError::NotInstalled)?;
        let spoken = all
            .get(&staying.conversation_id)
            .filter(|live| Arc::ptr_eq(&live.ear, &ear))
            .is_some_and(|live| {
                on_start(live.resident.pid());
                live.resident.say(turn.prompt)
            });
        if !spoken {
            // It exited since it last spoke, or was closed while this turn
            // waited: a new one, told to listen.
            if let Some(old) = all.remove(&staying.conversation_id) {
                old.resident.close();
            }
            let live = start(&staying, turn, started_as(turn))?;
            live.ear.claim(&tell);
            on_start(live.resident.pid());
            let said = live.resident.say(turn.prompt);
            all.insert(staying.conversation_id.clone(), live);
            if !said {
                return Err(AgentError::Unreadable(
                    "the conversation's process would not start".to_owned(),
                ));
            }
        }
    }
    loop {
        match heard.recv() {
            Ok(Heard::Part(part)) => on_part(part),
            Ok(Heard::Ended(said)) => return Ok(said),
            // Control answers are taken before they get here.
            Ok(Heard::Control(_)) => {}
            Ok(Heard::Gone) | Err(_) => {
                return Err(AgentError::Unreadable(
                    "the conversation's process ended".to_owned(),
                ))
            }
        }
    }
}

/// The residents a new one in `project` closes. One conversation of an
/// orchestrator listens at a time — two would both answer the same message —
/// but a chat asked to be reachable keeps its process: it is its claude.ai
/// link, and only turning Remote Control off lets it go.
pub(crate) fn evicted<'a>(
    listening: impl Iterator<Item = (&'a str, &'a str)>,
    project: &str,
    reachable: impl Fn(&str) -> bool,
) -> Vec<String> {
    listening
        .filter(|(key, of)| *of == project && !reachable(key))
        .map(|(key, _)| key.to_owned())
        .collect()
}

/// Sends a control request to this conversation's process. False when it has
/// none running.
pub(crate) fn control(
    app: &AppHandle,
    conversation_id: &str,
    id: &str,
    request: serde_json::Value,
) -> bool {
    app.try_state::<Residents>()
        .and_then(|residents| {
            residents.0.lock().ok().and_then(|all| {
                all.get(conversation_id)
                    .map(|live| live.resident.control(id, request))
            })
        })
        .unwrap_or(false)
}

/// Hands this conversation's process a message for the turn it is on. `None`
/// when it has no process, so the caller reaches the turn the other way.
pub(crate) fn steer(app: &AppHandle, conversation_id: &str, prompt: &str) -> Option<bool> {
    let residents = app.try_state::<Residents>()?;
    let all = residents.0.lock().ok()?;
    all.get(conversation_id)
        .map(|live| live.resident.say(prompt))
}

/// Ends this conversation's process, if it has one: a chat no longer
/// reachable goes back to a process per turn.
pub(crate) fn close(app: &AppHandle, conversation_id: &str) {
    let gone = app.try_state::<Residents>().and_then(|residents| {
        residents
            .0
            .lock()
            .ok()
            .and_then(|mut all| all.remove(conversation_id))
    });
    if let Some(live) = gone {
        live.resident.close();
    }
}

/// Stops the person's turn and keeps the process: the one waiting for a woken
/// turn stops waiting, and the woken turn goes on. False when this
/// conversation has none, so the caller stops it the other way.
pub(crate) fn interrupt(app: &AppHandle, conversation_id: &str) -> bool {
    app.try_state::<Residents>()
        .and_then(|residents| {
            residents.0.lock().ok().and_then(|all| {
                all.get(conversation_id)
                    .map(|live| live.ear.stop_waiting() || live.resident.interrupt())
            })
        })
        .unwrap_or(false)
}

fn started_as(turn: &Say<'_>) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{}|{}",
        turn.command,
        turn.model,
        turn.permission,
        turn.effort,
        turn.cwd.display(),
        turn.add_dirs.join(":")
    )
}

/// A person's turn stopped before it had the process: nothing was said.
fn stopped() -> Said {
    Said {
        end: devpit_rpc::TurnEnd {
            turn_id: String::new(),
            cost_usd: None,
            duration_ms: None,
            stop_reason: Some("cancelled".to_owned()),
            is_error: false,
            context: None,
        },
        session_id: None,
        init: None,
        anchor: None,
    }
}

fn start(staying: &Staying, turn: &Say<'_>, started_as: String) -> Result<Live, AgentError> {
    let driver =
        devpit_agentcli::driver::driver(&staying.driver).ok_or(AgentError::NotInstalled)?;
    let ear = Arc::new(Ear::default());
    let heard_by = ear.clone();
    let app = staying.app.clone();
    let conversation = staying.conversation_id.clone();
    let transcript = staying.transcript.clone();
    let head = staying.head.clone();
    let answers = staying.app.clone();
    // Where the CLI logs this conversation: who woke a turn is written there.
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_default();
    let config_dir = turn
        .env
        .iter()
        .find(|(key, _)| key == "CLAUDE_CONFIG_DIR")
        .map(|(_, value)| value.clone());
    let logs = devpit_agentcli::peers::logs_of(
        &devpit_agentcli::cli_config::config_dir_from(&home, config_dir.as_deref()),
        &turn
            .cwd
            .canonicalize()
            .unwrap_or_else(|_| turn.cwd.to_path_buf()),
    );
    let resident = Resident::start(driver, turn, Arc::new(|_: &str| {}), move |heard| {
        if let Heard::Control(said) = heard {
            crate::chat_remote::answered(&answers, said);
            return;
        }
        heard_by.hear(
            heard,
            || woke(&app, &conversation, &logs),
            &transcript,
            &head,
        );
    })?;
    Ok(Live {
        resident,
        started_as,
        project_id: staying.project_id.clone(),
        ear,
    })
}

/// A turn nobody here asked for has begun: opened in the conversation and
/// announced, so an open chat joins it through the relay.
fn woke(app: &AppHandle, conversation: &str, logs: &std::path::Path) -> Woken {
    let woken_by = Part::Text {
        text: crate::delegations::woken_label(logs),
        parent: None,
    };
    let relay = app.state::<crate::chat_relay::Relay>();
    let (frames, relaying) = relay.open(conversation, Channel::new(|_| Ok(())));
    let turn_id = format!("turn_{}", ulid::Ulid::generate());
    let answer_id = format!("msg_{}", ulid::Ulid::generate());
    let opened = Message {
        id: answer_id.clone(),
        turn_id: Some(turn_id.clone()),
        role: Role::Assistant,
        parts: vec![woken_by.clone()],
        created_at: crate::chat_listening::now(),
        streaming: true,
    };
    frames.send(Frame::Opened { message: opened }).ok();
    let _ = app.emit(WOKE, conversation);
    Woken {
        turn_id,
        answer_id,
        parts: vec![woken_by],
        frames,
        _relaying: relaying,
    }
}

#[cfg(test)]
#[path = "chat_resident_tests.rs"]
mod tests;
