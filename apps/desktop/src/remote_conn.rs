//! One viewer's connection: the hello that names its device, then requests
//! answered one by one, each checked against what the device may do.
//!
//! What a device may do is fixed for the connection. Changing it on the
//! machine drops the device's connections, and the viewer connects again
//! with what it has now — so a terminal watched read-only is attached again
//! read-only or not, by tmux, rather than trusted to the page.

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
const HELLO_WITHIN: Duration = Duration::from_secs(10);
/// The most a message from a viewer may be.
const MOST_MESSAGE: usize = 256 * 1024;
/// A connection that says nothing for this long is closed.
const IDLE: Duration = Duration::from_secs(90);

pub(crate) fn run(app: tauri::AppHandle, mut ws: WebSocket<TcpStream>, root: std::path::PathBuf) {
    let _ = ws
        .get_ref()
        .set_read_timeout(Some(Duration::from_millis(40)));
    let Some(device) = hello(&mut ws, &root) else {
        let _ = ws.close(None);
        let _ = ws.flush();
        return;
    };
    let (id, out, heard) = crate::remote_hub::join(&app, &device.id);
    crate::remote::note(&root, &device, "connected");
    let mut watching = Watching::new(id);
    let mut last = Instant::now();
    let host = std::env::var("HOSTNAME")
        .ok()
        .or_else(hostname)
        .unwrap_or_else(|| "this machine".to_owned());
    send(
        &mut ws,
        &RemoteOut::Welcome {
            device: device.name.clone(),
            host,
            typing: device.typing,
            answering: device.answering,
        },
    );
    loop {
        if !crate::remote_hub::holds(id) {
            break;
        }
        match ws.read() {
            Ok(Message::Text(text)) => {
                last = Instant::now();
                if text.len() > MOST_MESSAGE {
                    break;
                }
                match serde_json::from_str::<RemoteIn>(&text) {
                    Ok(asked) => {
                        if let Some(said) = answer(&app, &root, &device, &mut watching, &out, asked)
                        {
                            send(&mut ws, &said);
                        }
                    }
                    Err(_) => send(
                        &mut ws,
                        &RemoteOut::Failed {
                            why: "that is not a message devpit reads".to_owned(),
                        },
                    ),
                }
            }
            Ok(Message::Ping(_) | Message::Pong(_)) => last = Instant::now(),
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => break,
        }
        if last.elapsed() > IDLE {
            break;
        }
        while let Ok(said) = heard.try_recv() {
            send(&mut ws, &said);
        }
        let _ = ws.flush();
    }
    watching.close_all();
    crate::remote_hub::leave(&app, id);
    crate::remote::note(&root, &device, "disconnected");
    let _ = ws.close(None);
    let _ = ws.flush();
}

fn hostname() -> Option<String> {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|name| name.trim().to_owned())
        .filter(|name| !name.is_empty())
}

fn send(ws: &mut WebSocket<TcpStream>, said: &RemoteOut) {
    if let Ok(text) = serde_json::to_string(said) {
        let _ = ws.write(Message::text(text));
    }
}

/// The device a connection's first message names, or nothing — said why.
fn hello(ws: &mut WebSocket<TcpStream>, root: &std::path::Path) -> Option<Device> {
    let began = Instant::now();
    while began.elapsed() < HELLO_WITHIN {
        match ws.read() {
            Ok(Message::Text(text)) => {
                let Ok(RemoteIn::Hello { token }) = serde_json::from_str::<RemoteIn>(&text) else {
                    send(
                        ws,
                        &RemoteOut::Refused {
                            why: "say hello first".to_owned(),
                        },
                    );
                    return None;
                };
                let mut devices = Devices::read(root);
                let Some(device) = devices.by_token(&token).cloned() else {
                    send(
                        ws,
                        &RemoteOut::Refused {
                            why: "this device is not paired with this machine — pair it again"
                                .to_owned(),
                        },
                    );
                    return None;
                };
                devices.seen(&device.id, crate::remote::now());
                let _ = devices.write(root);
                return Some(device);
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(_) => return None,
        }
    }
    None
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
        RemoteIn::Hello { .. } => None,
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
