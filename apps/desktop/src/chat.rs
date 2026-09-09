//! The chat commands: send, cancel, and what was said before.
//!
//! The stream goes over a Channel, like the pty, because a turn is watched
//! rather than awaited.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use devpit_agentcli::driver::driver;
use devpit_agentcli::head::{head_path, read_head, settled, write_head, Head};
use devpit_agentcli::profile::profiles;
use devpit_agentcli::store::{append, conversation_path, read};
use devpit_agentcli::talk::{say, Say};
use devpit_rpc::{Ask, Conversation, ErrorCode, Frame, Message, Part, Role, RpcError, TurnEnd};
use tauri::ipc::Channel;
use tauri::State;

/// The turns in flight, by conversation, so one can be stopped.
#[derive(Default)]
pub struct Talking {
    running: Arc<Mutex<HashMap<String, u32>>>,
}

fn home() -> PathBuf {
    devpit_core::Store::root().unwrap_or_default()
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as f64)
        .unwrap_or_default()
}

fn id(prefix: &str) -> String {
    format!("{prefix}_{}", ulid::Ulid::generate())
}

/// `chat.history` — everything said in this conversation, in order.
#[tauri::command]
#[specta::specta]
pub fn chat_history(project_id: String, conversation_id: String) -> Result<Conversation, RpcError> {
    let home = home();
    let file = conversation_path(&home, &project_id, &conversation_id);
    let (messages, skipped) = read(&file);
    if skipped > 0 {
        // Reported rather than hidden: a conversation missing a line should
        // say so, not quietly show less than happened.
        eprintln!("{conversation_id}: {skipped} unreadable line(s)");
    }
    let head = read_head(&head_path(&home, &project_id, &conversation_id));
    Ok(Conversation {
        id: conversation_id,
        project_id,
        card_id: head.as_ref().and_then(|head| head.card_id.clone()),
        // Empty until the first turn settles it: a conversation nobody has
        // spoken in belongs to no account yet.
        profile: head
            .as_ref()
            .map(|head| head.profile.clone())
            .unwrap_or_default(),
        model: head.as_ref().and_then(|head| head.model.clone()),
        cost_usd: 0.0,
        created_at: head
            .map(|head| head.created_at)
            .or_else(|| messages.first().map(|first| first.created_at))
            .unwrap_or_else(now),
        messages,
    })
}

/// `chat.send` — one turn, streamed as it happens.
#[tauri::command]
#[specta::specta]
pub async fn chat_send(
    state: State<'_, Talking>,
    ask: Ask,
    on_frame: Channel<Frame>,
) -> Result<TurnEnd, RpcError> {
    let Ask {
        project_id,
        conversation_id,
        profile_id,
        model,
        prompt,
        cwd,
        budget_usd,
    } = ask;
    let home = home();
    let head_file = head_path(&home, &project_id, &conversation_id);
    let head = read_head(&head_file);
    if let Err(fixed) = settled(head.as_ref(), &profile_id) {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            format!("this conversation belongs to {fixed}"),
        ));
    }

    let Some(profile) = profiles(&[])
        .into_iter()
        .find(|found| found.id == profile_id)
    else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("no profile called {profile_id}"),
        ));
    };
    let Some(path) = profile.path.clone() else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("{} is not on the PATH", profile.command),
        ));
    };
    let Some(driver) = driver(&profile.driver) else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("no driver called {}", profile.driver),
        ));
    };

    let file = conversation_path(&home, &project_id, &conversation_id);
    let turn_id = id("turn");

    // Written before the turn runs: an interrupted first turn still leaves the
    // conversation tied to the account that opened it.
    let _ = write_head(
        &head_file,
        &Head {
            profile: profile_id.clone(),
            model: model.clone(),
            card_id: head.as_ref().and_then(|head| head.card_id.clone()),
            created_at: head.map(|head| head.created_at).unwrap_or_else(now),
        },
    );

    let asked = Message {
        id: id("msg"),
        turn_id: Some(turn_id.clone()),
        role: Role::User,
        parts: vec![Part::Text {
            text: prompt.clone(),
        }],
        created_at: now(),
        streaming: false,
    };
    let _ = append(&file, &asked);
    let _ = on_frame.send(Frame::Opened { message: asked });

    let answer_id = id("msg");
    let opened = Message {
        id: answer_id.clone(),
        turn_id: Some(turn_id.clone()),
        role: Role::Assistant,
        parts: Vec::new(),
        created_at: now(),
        streaming: true,
    };
    let _ = on_frame.send(Frame::Opened {
        message: opened.clone(),
    });

    let running = state.running.clone();
    let key = conversation_id.clone();
    let parts = Arc::new(Mutex::new(Vec::new()));
    let collected = parts.clone();
    let sink = on_frame.clone();
    let answer = answer_id.clone();

    let end = tauri::async_runtime::spawn_blocking(move || {
        say(
            driver.as_ref(),
            &Say {
                command: &path,
                prompt: &prompt,
                cwd: std::path::Path::new(&cwd),
                model: model.as_deref(),
                budget_usd,
                session_id: None,
            },
            |part| {
                collected
                    .lock()
                    .map(|mut held| held.push(part.clone()))
                    .ok();
                let _ = sink.send(Frame::Part {
                    message_id: answer.clone(),
                    part,
                });
            },
            |pid| {
                running
                    .lock()
                    .map(|mut held| held.insert(key.clone(), pid))
                    .ok();
            },
        )
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))?
    .map_err(|err| RpcError::internal(err.to_string()))?;

    state
        .running
        .lock()
        .map(|mut held| held.remove(&conversation_id))
        .ok();

    let said = Message {
        parts: parts.lock().map(|held| held.clone()).unwrap_or_default(),
        streaming: false,
        ..opened
    };
    let _ = append(&file, &said);

    let end = TurnEnd { turn_id, ..end };
    let _ = on_frame.send(Frame::Ended { end: end.clone() });
    Ok(end)
}

/// `chat.cancel` — stops the turn in flight, keeping what already arrived.
///
/// Answers with the ending it caused, or nothing when no turn was running.
#[tauri::command]
#[specta::specta]
pub fn chat_cancel(
    state: State<'_, Talking>,
    conversation_id: String,
) -> Result<Option<TurnEnd>, RpcError> {
    let pid = state
        .running
        .lock()
        .ok()
        .and_then(|held| held.get(&conversation_id).copied());
    let Some(pid) = pid else {
        return Ok(None);
    };
    // SIGTERM, not SIGKILL: the CLI gets to write its own last line.
    let _ = std::process::Command::new("kill")
        .arg("-TERM")
        .arg(pid.to_string())
        .status();
    Ok(Some(TurnEnd {
        turn_id: String::new(),
        cost_usd: None,
        duration_ms: None,
        stop_reason: Some("cancelled".to_owned()),
        is_error: false,
    }))
}

/// `agent.profiles` — the accounts this machine can talk to.
#[tauri::command]
#[specta::specta]
pub fn agent_profiles() -> Result<Vec<devpit_rpc::Profile>, RpcError> {
    Ok(profiles(&[]))
}

/// `chat.frames` — the shapes `chat.send` uses, on both sides.
///
/// It exists so the generated contract carries `Ask` and `Frame`: `chat.send`
/// streams over a Channel, which specta cannot describe, so it is left out of
/// the contract and its types would go with it.
#[tauri::command]
#[specta::specta]
pub fn chat_frames(_ask: Option<Ask>) -> Result<Vec<Frame>, RpcError> {
    Ok(Vec::new())
}
