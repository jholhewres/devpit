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
use std::time::Duration;

use devpit_agentcli::driver::Driver;
use devpit_agentcli::head::{read_head, write_head};
use devpit_agentcli::resident::{Heard, Resident};
use devpit_agentcli::store::append;
use devpit_agentcli::talk::{say, Said, Say};
use devpit_agentcli::AgentError;
use devpit_rpc::{Frame, Message, Part, Role};
use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager};

/// The event a woken turn is announced on, with its conversation id.
pub(crate) const WOKE: &str = "chat:woke";

/// What stays for one conversation.
struct Live {
    resident: Resident,
    /// The flags it was started with: a turn asking for others restarts it.
    started_as: String,
    project_id: String,
    listening: Arc<Mutex<Listening>>,
}

/// Where what the process says goes right now.
#[derive(Default)]
enum Listening {
    /// Nobody asked: a turn that starts now was woken.
    #[default]
    Nobody,
    /// The person's turn, read by the thread that sent it.
    Person(mpsc::Sender<Heard>),
    /// A turn another session woke, written here as it goes.
    Woken(Woken),
}

/// A woken turn in progress.
struct Woken {
    turn_id: String,
    answer_id: String,
    parts: Vec<Part>,
    frames: Channel<Frame>,
    _relaying: crate::chat_relay::Relaying,
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

/// What a turn needs to stay, when it is to: an orchestrator's, outside the
/// supervised mode.
pub(crate) fn staying(
    app: &AppHandle,
    project_id: &str,
    conversation_id: &str,
    sessions: &std::path::Path,
    driver: &str,
    mode: Option<&str>,
) -> Option<Staying> {
    stays(project_id, mode).then(|| Staying {
        app: app.clone(),
        project_id: project_id.to_owned(),
        conversation_id: conversation_id.to_owned(),
        transcript: devpit_agentcli::store::conversation_path(sessions, conversation_id),
        head: devpit_agentcli::head::head_path(sessions, conversation_id),
        driver: driver.to_owned(),
    })
}

fn stays(project_id: &str, mode: Option<&str>) -> bool {
    if mode == Some(crate::asking::ASKING_MODE) {
        return false;
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
        .and_then(|home| devpit_core::home::orchestrator_of(&home, &root))
        .is_some()
}

/// A turn, through the conversation's resident process when there is to be
/// one, and as a process of its own otherwise.
pub(crate) fn or_say(
    staying: Option<Staying>,
    driver: &dyn Driver,
    turn: &Say<'_>,
    mut on_part: impl FnMut(Part),
    mut on_start: impl FnMut(u32),
) -> Result<Said, AgentError> {
    let Some(staying) = staying else {
        return say(driver, turn, on_part, on_start);
    };
    let residents = staying.app.state::<Residents>();
    let (tell, heard) = mpsc::channel();
    {
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
        if !all.contains_key(&staying.conversation_id) {
            // One conversation of an orchestrator listens at a time: the one
            // last spoken in. Two would both answer the same message.
            let others: Vec<String> = all
                .iter()
                .filter(|(_, live)| live.project_id == staying.project_id)
                .map(|(key, _)| key.clone())
                .collect();
            for key in others {
                if let Some(old) = all.remove(&key) {
                    old.resident.close();
                }
            }
            let live = start(&staying, turn, wanted)?;
            all.insert(staying.conversation_id.clone(), live);
        }
        let live = all
            .get(&staying.conversation_id)
            .ok_or(AgentError::NotInstalled)?;
        let listening = live.listening.clone();
        // A woken turn still running finishes first: the person's words go
        // in after it, not into the middle of it.
        drop(all);
        wait_for_quiet(&listening);
        if let Ok(mut now) = listening.lock() {
            *now = Listening::Person(tell.clone());
        }
        let mut all = residents.0.lock().map_err(|_| AgentError::NotInstalled)?;
        let live = all
            .get(&staying.conversation_id)
            .ok_or(AgentError::NotInstalled)?;
        on_start(live.resident.pid());
        if !live.resident.say(turn.prompt) {
            // It exited since it last spoke: a new one, told to listen.
            all.remove(&staying.conversation_id);
            let live = start(&staying, turn, started_as(turn))?;
            if let Ok(mut now) = live.listening.lock() {
                *now = Listening::Person(tell);
            }
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
            Ok(Heard::Gone) | Err(_) => {
                return Err(AgentError::Unreadable(
                    "the conversation's process ended".to_owned(),
                ))
            }
        }
    }
}

/// Stops the turn in flight and keeps the process. False when this
/// conversation has none, so the caller stops it the other way.
pub(crate) fn interrupt(app: &AppHandle, conversation_id: &str) -> bool {
    app.try_state::<Residents>()
        .and_then(|residents| {
            residents.0.lock().ok().and_then(|all| {
                all.get(conversation_id)
                    .map(|live| live.resident.interrupt())
            })
        })
        .unwrap_or(false)
}

fn started_as(turn: &Say<'_>) -> String {
    format!(
        "{}|{:?}|{:?}|{:?}|{}",
        turn.command,
        turn.model,
        turn.permission,
        turn.effort,
        turn.cwd.display()
    )
}

fn wait_for_quiet(listening: &Mutex<Listening>) {
    // A woken turn is short — an answer to a message — but not bounded by
    // anything here; the person's turn waits for as long as it takes.
    while listening
        .lock()
        .is_ok_and(|now| !matches!(*now, Listening::Nobody))
    {
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn start(staying: &Staying, turn: &Say<'_>, started_as: String) -> Result<Live, AgentError> {
    let driver =
        devpit_agentcli::driver::driver(&staying.driver).ok_or(AgentError::NotInstalled)?;
    let listening = Arc::new(Mutex::new(Listening::Nobody));
    let heard_by = listening.clone();
    let app = staying.app.clone();
    let conversation = staying.conversation_id.clone();
    let transcript = staying.transcript.clone();
    let head = staying.head.clone();
    let resident = Resident::start(driver, turn, Arc::new(|_: &str| {}), move |heard| {
        let Ok(mut now) = heard_by.lock() else { return };
        match (&mut *now, heard) {
            (Listening::Person(tell), Heard::Ended(said)) => {
                let _ = tell.send(Heard::Ended(said));
                *now = Listening::Nobody;
            }
            // Gone mid-turn: that turn fails, and the conversation is left
            // quiet so the next one starts a new process instead of waiting
            // for this one for ever.
            (Listening::Person(tell), Heard::Gone) => {
                let _ = tell.send(Heard::Gone);
                *now = Listening::Nobody;
            }
            (Listening::Person(tell), heard) => {
                let _ = tell.send(heard);
            }
            (Listening::Nobody, Heard::Part(part)) => {
                let mut woken = woke(&app, &conversation);
                woken.heard(part);
                *now = Listening::Woken(woken);
            }
            (Listening::Woken(woken), Heard::Part(part)) => woken.heard(part),
            (Listening::Woken(_), Heard::Ended(said)) => {
                if let Listening::Woken(woken) = std::mem::take(&mut *now) {
                    woken.finish(&transcript, &head, said);
                }
            }
            (Listening::Woken(_), Heard::Gone) => {
                if let Listening::Woken(woken) = std::mem::take(&mut *now) {
                    woken.frames.send(Frame::Ended { end: gone() }).ok();
                }
            }
            (Listening::Nobody, _) => {}
        }
    })?;
    Ok(Live {
        resident,
        started_as,
        project_id: staying.project_id.clone(),
        listening,
    })
}

/// A turn nobody here asked for has begun: opened in the conversation and
/// announced, so an open chat joins it through the relay.
fn woke(app: &AppHandle, conversation: &str) -> Woken {
    let relay = app.state::<crate::chat_relay::Relay>();
    let (frames, relaying) = relay.open(conversation, Channel::new(|_| Ok(())));
    let turn_id = format!("turn_{}", ulid::Ulid::generate());
    let answer_id = format!("msg_{}", ulid::Ulid::generate());
    let opened = Message {
        id: answer_id.clone(),
        turn_id: Some(turn_id.clone()),
        role: Role::Assistant,
        parts: vec![WOKEN_BY.clone()],
        created_at: now(),
        streaming: true,
    };
    frames.send(Frame::Opened { message: opened }).ok();
    let _ = app.emit(WOKE, conversation);
    Woken {
        turn_id,
        answer_id,
        parts: vec![WOKEN_BY.clone()],
        frames,
        _relaying: relaying,
    }
}

/// Said at the top of a turn nobody here asked for, so it does not read as an
/// answer to something the person said.
static WOKEN_BY: std::sync::LazyLock<Part> = std::sync::LazyLock::new(|| Part::Text {
    text: "↪ *Not in answer to you — another session wrote, or work it left running finished.*\n\n"
        .to_owned(),
    parent: None,
});

impl Woken {
    fn heard(&mut self, part: Part) {
        self.parts.push(part.clone());
        self.frames
            .send(Frame::Part {
                message_id: self.answer_id.clone(),
                part,
            })
            .ok();
    }

    fn finish(self, transcript: &std::path::Path, head: &std::path::Path, said: Said) {
        let answered = Message {
            id: self.answer_id,
            turn_id: Some(self.turn_id.clone()),
            role: Role::Assistant,
            parts: self.parts,
            created_at: now(),
            streaming: false,
        };
        let _ = append(transcript, &answered);
        if let Some(opening) = read_head(head) {
            let after = crate::chat_turn::after(
                opening,
                &self.turn_id,
                &said.end,
                said.session_id,
                said.anchor,
            );
            let _ = write_head(head, &after);
        }
        let end = devpit_rpc::TurnEnd {
            turn_id: self.turn_id,
            ..said.end
        };
        self.frames.send(Frame::Ended { end }).ok();
    }
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or_default()
}

fn gone() -> devpit_rpc::TurnEnd {
    devpit_rpc::TurnEnd {
        turn_id: String::new(),
        cost_usd: None,
        duration_ms: None,
        stop_reason: Some("interrupted".to_owned()),
        is_error: true,
        context: None,
    }
}
