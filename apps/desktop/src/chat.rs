//! The chat commands: send, cancel, and what was said before.
//!
//! The stream goes over a Channel, like the pty, because a turn is watched
//! rather than awaited.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use devpit_agentcli::driver::driver;
use devpit_agentcli::head::{head_path, read_head, remaining, settled, write_head};
use devpit_agentcli::store::{append, conversation_path, read};
use devpit_agentcli::talk::{say, Said, Say};
use devpit_rpc::{Ask, Conversation, ErrorCode, Frame, Message, Part, Role, RpcError, TurnEnd};
use tauri::ipc::Channel;

use crate::card_activity::Doing;
use tauri::{Manager, State};

/// The turns in flight, by conversation, so one can be stopped.
#[derive(Default)]
pub struct Talking {
    /// The pid serving each conversation, `None` while its turn is still
    /// starting. A key here means a turn is claimed — see `Talking::begin`.
    pub(crate) running: Arc<Mutex<HashMap<String, Option<u32>>>>,
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
pub async fn chat_history(
    project_id: String,
    conversation_id: String,
) -> Result<Conversation, RpcError> {
    crate::off_main::blocking(move || chat_history_now(project_id, conversation_id)).await
}

/// [`chat_history`], on the calling thread.
pub(crate) fn chat_history_now(
    project_id: String,
    conversation_id: String,
) -> Result<Conversation, RpcError> {
    let sessions = crate::projects::project_home(&project_id)?.sessions();
    let file = conversation_path(&sessions, &conversation_id);
    let (messages, skipped) = read(&file);
    if skipped > 0 {
        // Reported rather than hidden: a conversation missing a line should
        // say so, not quietly show less than happened.
        eprintln!("{conversation_id}: {skipped} unreadable line(s)");
    }
    let head = read_head(&head_path(&sessions, &conversation_id));
    let messages =
        crate::adopted_history::or_backfilled(messages, head.as_ref(), &project_id, &file);
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
        permission: head.as_ref().and_then(|head| head.permission.clone()),
        effort: head.as_ref().and_then(|head| head.effort.clone()),
        cwd: head.as_ref().and_then(|head| head.cwd.clone()),
        session_id: head.as_ref().and_then(|head| head.session_id.clone()),
        cost_usd: head.as_ref().map(|head| head.cost_usd).unwrap_or_default(),
        context: head.as_ref().and_then(|head| head.context),
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
    relay: State<'_, crate::chat_relay::Relay>,
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
    // Every frame goes through the relay, so a chat reopened mid-turn can join.
    let (on_frame, _relaying) = relay.open(&conversation_id, on_frame);
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
    // Which account this turn spends. A terminal gets it as assignments in the
    // line it types and a step gets it from its runner; a chat was the one
    // path that spawned the binary with none of it.
    let env = devpit_agentcli::running::runner(&profile).env;
    let mcp = crate::agent_reach::chat_mcp(&profile.driver);

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
    let hold = crate::asking::holding(mode.as_deref(), &app, on_frame.clone());

    // Claimed before anything is spawned, and released by the guard however
    // this returns.
    // Nothing new once an update is about to go in: a turn started here would
    // die with the process at the commit, halfway through an answer.
    if let Some(updating) = app.try_state::<crate::update::Updating>() {
        if let Some(why) = crate::update::starting_refused(&updating.state()) {
            return Err(RpcError::new(ErrorCode::Busy, why.to_owned()));
        }
    }
    let guard = state.begin(&conversation_id, &steering)?;
    let control = steering.hold(&conversation_id);
    crate::card_chat::turn_heard(&app, &conversation_id, Doing::Working);
    let said = tauri::async_runtime::spawn_blocking(move || {
        let hooks = crate::steps::hook_settings();
        let checkout = std::path::Path::new(&cwd);
        let before = crate::turn_changes::before(checkout);
        let said = say(
            driver.as_ref(),
            &Say {
                command: &path,
                env: &env,
                prompt: &prompt,
                cwd: checkout,
                model: model.as_deref(),
                // What is left of the cap, not the cap: a resumed conversation
                // may not spend its whole budget again.
                budget_usd: left,
                session_id: resuming.as_deref(),
                fork_at: forking.as_deref(),
                permission: mode.as_deref(),
                settings: hooks.as_deref(),
                mcp_config: mcp.as_deref(),
                effort: thinking.as_deref(),
                control: Some(&control),
                on_session: Some(&hold),
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
                    .map(|mut held| held.insert(key.clone(), Some(pid)))
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

    // The guard does both when it goes, including on the `?` above.
    drop(guard);

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
    let head = crate::chat_turn::after(opening, &turn_id, &end, session_id, anchor);
    let _ = write_head(&head_file, &head);

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
