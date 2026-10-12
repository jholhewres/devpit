//! One viewer's connection: the device it is, then requests answered one by
//! one, each checked against what the device may do.
//!
//! What a device may do is fixed for the connection. Changing it on the
//! machine drops the device's connections, and the viewer connects again
//! with what it has now — so a terminal watched read-only is attached again
//! read-only or not, by tmux, rather than trusted to the page.
//!
//! The connection is a `Wire`: a WebSocket on the tailnet, where the device
//! says hello with its token, or a Noise session through the relay, where
//! the handshake already proved which device it is.

use std::net::TcpStream;
use std::time::{Duration, Instant};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use devpit_rpc::{RemoteIn, RemoteOut, RemoteProject, RemoteTerminal};
use tauri::Manager as _;
use tungstenite::{Message, WebSocket};

use crate::remote_devices::{Device, Devices};
use crate::remote_panes::Watching;

/// How long a viewer has to say who it is.
pub(crate) const HELLO_WITHIN: Duration = Duration::from_secs(10);
/// The most a message from a viewer may be.
pub(crate) const MOST_MESSAGE: usize = 256 * 1024;
/// A connection that says nothing for this long is closed.
const IDLE: Duration = Duration::from_secs(90);

/// What one read of a connection brought.
pub(crate) enum Heard {
    Text(String),
    /// A ping, or part of a message: the viewer is there.
    Alive,
    /// Nothing within the read's timeout.
    Nothing,
}

/// A viewer's connection, however it came.
pub(crate) trait Wire {
    /// One read; an error when the connection is gone.
    fn hear(&mut self) -> Result<Heard, ()>;
    fn say(&mut self, text: &str);
    /// Ends the connection, and with it whatever keys it held.
    fn end(&mut self);
}

/// The tailnet's: one JSON message per WebSocket frame, on the machine's own
/// TLS or the tailnet's encryption.
pub(crate) struct Tailnet(pub WebSocket<TcpStream>);

impl Wire for Tailnet {
    fn hear(&mut self) -> Result<Heard, ()> {
        match self.0.read() {
            Ok(Message::Text(text)) => Ok(Heard::Text(text.to_string())),
            Ok(Message::Ping(_) | Message::Pong(_)) => Ok(Heard::Alive),
            Ok(Message::Close(_)) => Err(()),
            Ok(_) => Ok(Heard::Nothing),
            Err(tungstenite::Error::Io(err))
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(Heard::Nothing)
            }
            Err(_) => Err(()),
        }
    }

    fn say(&mut self, text: &str) {
        let _ = self.0.send(Message::text(text));
    }

    fn end(&mut self) {
        let _ = self.0.close(None);
        let _ = self.0.flush();
    }
}

/// A tailnet viewer: its hello names the device, then it is served.
pub(crate) fn run(app: tauri::AppHandle, ws: WebSocket<TcpStream>, root: std::path::PathBuf) {
    let _ = ws
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(40)));
    let mut wire = Tailnet(ws);
    match hello(&mut wire, &root) {
        Some(device) => serve(app, wire, root, device),
        None => wire.end(),
    }
}

/// `device`'s requests, answered until it leaves, goes silent or is dropped
/// here.
pub(crate) fn serve(
    app: tauri::AppHandle,
    mut wire: impl Wire,
    root: std::path::PathBuf,
    device: Device,
) {
    let (id, out, heard) = crate::remote_hub::join(&app, &device.id);
    crate::remote::note(&root, &device, "connected");
    let mut watching = Watching::new(id);
    let mut last = Instant::now();
    send(
        &mut wire,
        &RemoteOut::Welcome {
            device: device.name.clone(),
            host: machine_name(),
            typing: device.typing,
            answering: device.answering,
        },
    );
    loop {
        if !crate::remote_hub::holds(id) {
            break;
        }
        match wire.hear() {
            Ok(Heard::Text(text)) => {
                last = Instant::now();
                if text.len() > MOST_MESSAGE {
                    break;
                }
                match serde_json::from_str::<RemoteIn>(&text) {
                    Ok(asked) => {
                        if let Some(said) = answer(&app, &root, &device, &mut watching, &out, asked)
                        {
                            send(&mut wire, &said);
                        }
                    }
                    Err(_) => send(
                        &mut wire,
                        &RemoteOut::Failed {
                            why: "that is not a message devpit reads".to_owned(),
                        },
                    ),
                }
            }
            Ok(Heard::Alive) => last = Instant::now(),
            Ok(Heard::Nothing) => {}
            Err(()) => break,
        }
        if last.elapsed() > IDLE {
            break;
        }
        while let Ok(said) = heard.try_recv() {
            send(&mut wire, &said);
        }
    }
    watching.close_all();
    crate::remote_hub::leave(&app, id);
    crate::remote::note(&root, &device, "disconnected");
    wire.end();
}

/// What this machine is called, as a viewer shows it.
pub(crate) fn machine_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .or_else(hostname)
        .unwrap_or_else(|| "this machine".to_owned())
}

fn hostname() -> Option<String> {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

pub(crate) fn send(wire: &mut impl Wire, said: &RemoteOut) {
    if let Ok(text) = serde_json::to_string(said) {
        wire.say(&text);
    }
}

pub(crate) fn refuse(wire: &mut impl Wire, why: &str) {
    send(
        wire,
        &RemoteOut::Refused {
            why: why.to_owned(),
        },
    );
}

/// The first message a viewer sends, within `HELLO_WITHIN`.
pub(crate) fn first(wire: &mut impl Wire) -> Option<RemoteIn> {
    let began = Instant::now();
    while began.elapsed() < HELLO_WITHIN {
        match wire.hear() {
            Ok(Heard::Text(text)) if text.len() <= MOST_MESSAGE => {
                return serde_json::from_str::<RemoteIn>(&text).ok();
            }
            Ok(Heard::Text(_)) | Err(()) => return None,
            Ok(Heard::Alive | Heard::Nothing) => {}
        }
    }
    None
}

/// The device a tailnet connection's hello names, or nothing — said why.
fn hello(wire: &mut impl Wire, root: &std::path::Path) -> Option<Device> {
    let Some(RemoteIn::Hello { token }) = first(wire) else {
        refuse(wire, "say hello first");
        return None;
    };
    let mut devices = Devices::read(root);
    let Some(device) = devices.by_token(&token).cloned() else {
        refuse(
            wire,
            "this device is not paired with this machine — pair it again",
        );
        return None;
    };
    devices.seen(&device.id, crate::remote::now());
    let _ = devices.write(root);
    Some(device)
}

fn drafts_now() -> RemoteOut {
    RemoteOut::Drafts {
        drafts: crate::reply_drafts::all()
            .into_iter()
            .map(|(profile, name, text)| devpit_rpc::RemoteDraft {
                profile,
                name,
                text,
            })
            .collect(),
    }
}

fn failed(why: impl Into<String>) -> Option<RemoteOut> {
    Some(RemoteOut::Failed { why: why.into() })
}

/// What a request gets, checked against what `device` may do.
fn answer(
    app: &tauri::AppHandle,
    root: &std::path::Path,
    device: &Device,
    watching: &mut Watching,
    out: &std::sync::mpsc::Sender<RemoteOut>,
    asked: RemoteIn,
) -> Option<RemoteOut> {
    match asked {
        // Who the device is was settled before its requests were read.
        RemoteIn::Hello { .. } | RemoteIn::Pair { .. } => None,
        RemoteIn::Ping => Some(RemoteOut::Pong),
        RemoteIn::Projects => Some(match projects() {
            Ok(projects) => RemoteOut::Projects { projects },
            Err(why) => RemoteOut::Failed { why },
        }),
        RemoteIn::PaneOpen {
            project,
            pane,
            cols,
            rows,
        } => match watching.open(&project, &pane, device.typing, (cols, rows), out.clone()) {
            Ok(()) => {
                crate::remote::note(root, device, &format!("watching {pane}"));
                None
            }
            Err(why) => failed(why),
        },
        RemoteIn::PaneInput { pane, b64 } => {
            if !device.typing {
                return failed("this device may only watch — allow it to type on the machine");
            }
            let Ok(bytes) = BASE64.decode(b64) else {
                return failed("those keys are not base64");
            };
            watching.input(&pane, &bytes).err().and_then(failed)
        }
        RemoteIn::PanePaste { pane, text } => {
            if !device.typing {
                return failed("this device may only watch — allow it to type on the machine");
            }
            let text: String = text
                .chars()
                .filter(|c| *c == '\n' || !c.is_control())
                .take(4000)
                .collect();
            if text.trim().is_empty() {
                return None;
            }
            let Some(project) = watched_project(&pane) else {
                return failed("that terminal is not open");
            };
            let sent = crate::sessions::tmux_server()
                .map_err(|err| err.message)
                .and_then(|server| {
                    let target = devpit_tmux::Server::target(
                        &devpit_tmux::Server::session_name(&project),
                        &pane,
                    );
                    server
                        .paste_and_send(&target, &text)
                        .map_err(|err| err.to_string())
                });
            crate::remote::note(root, device, &format!("sent a line to {pane}"));
            sent.err().and_then(failed)
        }
        RemoteIn::PaneClose { pane } => {
            watching.close(&pane);
            None
        }
        RemoteIn::Board { project } => Some(match crate::board::board_get_now(project.clone()) {
            Ok(board) => RemoteOut::Board {
                project,
                board: Box::new(board),
            },
            Err(err) => RemoteOut::Failed { why: err.message },
        }),
        RemoteIn::CardMove {
            project,
            card,
            column,
        } => {
            if !device.typing {
                return failed("this device may only watch — allow it to type on the machine");
            }
            let board = match crate::board::board_get_now(project) {
                Ok(board) => board,
                Err(err) => return failed(err.message),
            };
            match crate::agent_api::moved(Some(app), &board, &card, &column) {
                Ok(_) => {
                    crate::remote::note(root, device, &format!("moved {card}"));
                    None
                }
                Err(why) => failed(why),
            }
        }
        RemoteIn::Waiting => Some(RemoteOut::Waiting {
            questions: crate::remote_hub::questions(),
        }),
        RemoteIn::Answer { id, allow } => {
            if !device.answering {
                return failed("this device may not answer — allow it on the machine");
            }
            crate::remote::note(
                root,
                device,
                if allow {
                    "allowed a tool"
                } else {
                    "denied a tool"
                },
            );
            settle(app, &id, allow).err().and_then(failed)
        }
        RemoteIn::Chats { project } => Some(match crate::threads::chat_list_now(project.clone()) {
            Ok(conversations) => RemoteOut::Chats {
                project,
                conversations: Box::new(conversations),
            },
            Err(err) => RemoteOut::Failed { why: err.message },
        }),
        RemoteIn::Chat {
            project,
            conversation,
        } => Some(
            match crate::chat::chat_history_now(project.clone(), conversation) {
                Ok(conversation) => RemoteOut::Chat {
                    project,
                    conversation: Box::new(conversation),
                },
                Err(err) => RemoteOut::Failed { why: err.message },
            },
        ),
        RemoteIn::Drafts => Some(drafts_now()),
        RemoteIn::DraftSend { profile, name } => {
            if !device.typing {
                return failed("this device may only watch — allow it to type on the machine");
            }
            let Some(text) = crate::reply_drafts::drafted(&profile, &name) else {
                return failed("that draft is no longer waiting");
            };
            if let Err(err) =
                crate::live_sessions::reply_now(&profile, &name, &text, Some("the Remote"))
            {
                return failed(err.message);
            }
            crate::remote::note(root, device, &format!("sent a draft to {name}"));
            Some(drafts_now())
        }
        RemoteIn::CardCreate { project, title } => {
            if !device.typing {
                return failed("this device may only watch — allow it to type on the machine");
            }
            crate::remote::note(root, device, "created a card");
            crate::remote_start::card(app, &project, &title)
                .err()
                .and_then(failed)
        }
        RemoteIn::SessionStart {
            project,
            card,
            prompt,
            anyway,
        } => {
            if !device.typing {
                return failed("this device may only watch — allow it to type on the machine");
            }
            Some(
                match crate::remote_start::session(app, &project, card.as_deref(), &prompt, anyway)
                {
                    Ok(name) => {
                        crate::remote::note(root, device, &format!("started {name}"));
                        RemoteOut::Started { name }
                    }
                    Err(why) => RemoteOut::Failed { why },
                },
            )
        }
        RemoteIn::SecretSet {
            project,
            name,
            value,
        } => Some(
            match crate::projects::store()
                .map_err(|err| err.message)
                .and_then(|store| {
                    crate::remote_secret::keep(
                        &store,
                        root,
                        &device.id,
                        &project,
                        &name,
                        &value,
                        crate::remote::now(),
                    )
                }) {
                Ok(name) => {
                    crate::vault_rotation::told(app, &project);
                    RemoteOut::SecretKept { project, name }
                }
                Err(why) => RemoteOut::Failed { why },
            },
        ),
        RemoteIn::AgentHealth => Some(RemoteOut::AgentHealth {
            health: crate::agent_door::health_now(app),
        }),
        RemoteIn::AgentRestart => {
            if !device.typing {
                return failed("this device may only watch — allow it to type on the machine");
            }
            crate::remote::note(root, device, "restarted devpit's MCP");
            Some(match crate::agent_door::restart_by_hand(app) {
                Ok(health) => RemoteOut::AgentHealth { health },
                Err(why) => RemoteOut::Failed { why },
            })
        }
    }
}

/// A question answered as the person: a terminal's held one, or a chat's.
fn settle(app: &tauri::AppHandle, id: &str, allow: bool) -> Result<(), String> {
    if id.starts_with("ask_") {
        let verdict = if allow {
            devpit_rpc::IslandVerdict::Allow
        } else {
            devpit_rpc::IslandVerdict::Deny
        };
        return crate::pane_asking::decide(id, verdict);
    }
    let answer = if allow {
        crate::asking::Answer::Allow
    } else {
        crate::asking::Answer::Deny
    };
    let asking = app
        .try_state::<crate::asking::Asking>()
        .ok_or("nothing is asking")?;
    asking
        .answer(id, answer)
        .then_some(())
        .ok_or_else(|| "that question is no longer waiting".to_owned())
}

/// The project a pane belongs to, by its session.
fn watched_project(pane: &str) -> Option<String> {
    let projects = crate::projects::project_list_now().ok()?.projects;
    let server = crate::sessions::tmux_server().ok()?;
    projects.into_iter().map(|one| one.id).find(|id| {
        server
            .running(&devpit_tmux::Server::session_name(id))
            .is_ok_and(|running| running.iter().any(|one| one.leaf_id == pane))
    })
}

/// Every project and its open terminals.
fn projects() -> Result<Vec<RemoteProject>, String> {
    let listed = crate::projects::project_list_now()
        .map_err(|err| err.message)?
        .projects;
    let server = crate::sessions::tmux_server().ok();
    Ok(listed
        .into_iter()
        .filter(|one| one.orchestrator.is_none())
        .map(|one| {
            let terminals = server
                .as_ref()
                .and_then(|server| {
                    server
                        .running(&devpit_tmux::Server::session_name(&one.id))
                        .ok()
                })
                .unwrap_or_default()
                .into_iter()
                .map(|running| RemoteTerminal {
                    pane: running.leaf_id,
                    command: running.command,
                })
                .collect();
            RemoteProject {
                id: one.id,
                name: one.name,
                group: one.group,
                terminals,
            }
        })
        .collect())
}
