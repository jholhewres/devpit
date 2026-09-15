//! The chat commands: send, cancel, and what was said before.
//!
//! The stream goes over a Channel, like the pty, because a turn is watched
//! rather than awaited.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use devpit_agentcli::driver::driver;
use devpit_agentcli::head::{after_turn, head_path, read_head, remaining, settled, write_head};
use devpit_agentcli::store::{append, conversation_path, read};
use devpit_agentcli::talk::{say, Said, Say};
use devpit_rpc::{
    Ask, Attachment, Conversation, ErrorCode, Frame, Message, Part, Role, RpcError, TurnEnd,
};
use tauri::ipc::Channel;

use crate::card_activity::Doing;
use tauri::State;

/// The turns in flight, by conversation, so one can be stopped.
#[derive(Default)]
pub struct Talking {
    pub(crate) running: Arc<Mutex<HashMap<String, u32>>>,
}

pub(crate) fn home() -> PathBuf {
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
    let sessions = crate::projects::project_home(&project_id)?.sessions();
    let file = conversation_path(&sessions, &conversation_id);
    let (messages, skipped) = read(&file);
    if skipped > 0 {
        // Reported rather than hidden: a conversation missing a line should
        // say so, not quietly show less than happened.
        eprintln!("{conversation_id}: {skipped} unreadable line(s)");
    }
    let head = read_head(&head_path(&sessions, &conversation_id));
    let card = crate::card_chat::conversation_card(&crate::projects::store()?, &conversation_id)?;
    Ok(Conversation {
        id: conversation_id,
        project_id,
        card_id: card.id,
        card_title: card.title,
        card_on_board: card.on_board,
        // Empty until the first turn settles it: a conversation nobody has
        // spoken in belongs to no account yet.
        profile: head
            .as_ref()
            .map(|head| head.profile.clone())
            .unwrap_or_default(),
        model: head.as_ref().and_then(|head| head.model.clone()),
        session_id: head.as_ref().and_then(|head| head.session_id.clone()),
        cost_usd: head.as_ref().map(|head| head.cost_usd).unwrap_or_default(),
        rewindable: head
            .as_ref()
            .map(|head| {
                head.rewind
                    .anchors
                    .iter()
                    .map(|at| at.turn_id.clone())
                    .collect()
            })
            .unwrap_or_default(),
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
    app: tauri::AppHandle,
    state: State<'_, Talking>,
    steering: State<'_, crate::steering::Steering>,
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
        permission,
        effort,
    } = ask;
    let home = home();
    let sessions = crate::projects::project_home(&project_id)?.sessions();
    let head_file = head_path(&sessions, &conversation_id);
    let head = read_head(&head_file);
    if let Err(fixed) = settled(head.as_ref(), &profile_id) {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            format!("this conversation belongs to {fixed}"),
        ));
    }
    // Refused here rather than half way through: a cap that only stops a turn
    // mid-answer is not a ceiling.
    if remaining(head.as_ref()).is_some_and(|left| left <= 0.0) {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "this conversation has spent its budget".to_owned(),
        ));
    }
    let cwd = crate::chat_turn::turn_cwd(head.as_ref().and_then(|head| head.cwd.as_deref()), &cwd)?;

    let Some(profile) = crate::agent_profiles::all(&crate::projects::store()?)?
        .into_iter()
        .find(|found| found.id == profile_id)
    else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("no profile called {profile_id}"),
        ));
    };
    // Spawned, not typed, so a name only the shell knows is no use here — and
    // saying so beats "not on the PATH" about something the person watches
    // their terminal run every day.
    let Some(path) = profile.path.clone() else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            match profile.reach {
                devpit_rpc::Reach::ShellOnly => format!(
                    "{} is a shell function, so devpit cannot start it here — \
                     name the program it runs instead",
                    profile.command
                ),
                _ => format!("{} is not installed", profile.command),
            },
        ));
    };
    let Some(driver) = driver(&profile.driver) else {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("no driver called {}", profile.driver),
        ));
    };

    let file = conversation_path(&sessions, &conversation_id);
    let turn_id = id("turn");

    // Written before the turn runs: an interrupted first turn still leaves the
    // conversation tied to the account that opened it.
    let opening = crate::chat_turn::opening(
        head.as_ref(),
        &profile_id,
        model.clone(),
        budget_usd,
        permission,
        effort,
        now(),
    );
    let _ = write_head(&head_file, &opening);
    let resuming = opening.session_id.clone();
    let forking = opening.rewind.fork_at.clone();
    let mode = opening.permission.clone();
    let thinking = opening.effort.clone();
    let left = remaining(Some(&opening));

    let asked = Message {
        id: id("msg"),
        turn_id: Some(turn_id.clone()),
        role: Role::User,
        parts: vec![Part::Text {
            text: prompt.clone(),
            parent: None,
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

    let control = steering.hold(&conversation_id);
    crate::card_chat::turn_heard(&app, &conversation_id, Doing::Working);
    let said = tauri::async_runtime::spawn_blocking(move || {
        let checkout = std::path::Path::new(&cwd);
        let before = crate::turn_changes::before(checkout);
        let said = say(
            driver.as_ref(),
            &Say {
                command: &path,
                prompt: &prompt,
                cwd: checkout,
                model: model.as_deref(),
                // What is left of the cap, not the cap: a resumed conversation
                // may not spend its whole budget again.
                budget_usd: left,
                session_id: resuming.as_deref(),
                fork_at: forking.as_deref(),
                permission: mode.as_deref(),
                effort: thinking.as_deref(),
                control: Some(&control),
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
        );
        crate::turn_changes::report(checkout, before.as_deref(), &collected, &sink, &answer);
        said
    })
    .await;
    // Before the answer is looked at: a turn that failed is not still working.
    crate::card_chat::turn_heard(&app, &conversation_id, Doing::Done);
    let said = said
        .map_err(|err| RpcError::internal(err.to_string()))?
        .map_err(|err| RpcError::internal(err.to_string()))?;

    state
        .running
        .lock()
        .map(|mut held| held.remove(&conversation_id))
        .ok();
    steering.release(&conversation_id);

    let answered = Message {
        parts: parts.lock().map(|held| held.clone()).unwrap_or_default(),
        streaming: false,
        ..opened
    };
    let _ = append(&file, &answered);

    let Said {
        end,
        session_id,
        init,
        anchor,
    } = said;
    crate::slash::remember(&home, &profile_id, init.as_ref());
    let _ = write_head(
        &head_file,
        &after_turn(opening, &turn_id, end.cost_usd, session_id, anchor),
    );

    let end = TurnEnd { turn_id, ..end };
    let _ = on_frame.send(Frame::Ended { end: end.clone() });
    Ok(end)
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

/// `chat.attach` — a dropped file, as something the agent can be pointed at.
///
/// The absolute path never reaches the screen or the prompt: it says nothing
/// on another machine, and a path outside the project is refused here rather
/// than read.
#[tauri::command]
#[specta::specta]
pub fn chat_attach(project_id: String, path: String) -> Result<Attachment, RpcError> {
    let store = crate::projects::store()?;
    let (_, root) = crate::projects::locate(&store, &project_id)?;
    let absolute = PathBuf::from(&path);
    let relative = devpit_core::tree::relative_to(&root, &absolute)
        .map_err(|_| RpcError::new(ErrorCode::Invalid, "that file is not in the project"))?;
    Ok(Attachment {
        name: absolute
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| relative.clone()),
        kind: absolute
            .extension()
            .map(|ext| ext.to_string_lossy().to_lowercase())
            .unwrap_or_default(),
        path: relative,
    })
}
