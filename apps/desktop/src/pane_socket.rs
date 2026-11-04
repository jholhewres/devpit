//! `GET /panes`: every terminal's bytes on one binary WebSocket per page.
//!
//! The desktop window opens it too: a WebSocket frame is far cheaper in
//! WebKit than the `eval` and `fetch` a Tauri channel costs per frame. The
//! window's ticket comes over its own IPC ([`crate::panes::panes_socket`]); a
//! browser comes in with its cookie.
//!
//! A frame is `[u32 handle][u8 kind][bytes]`, little-endian. The handle is
//! the page's number for one attach; `web/src/shell/paneSocket.ts` is the
//! other end and keeps the same kinds. Output is acknowledged by the page once
//! xterm parsed it, which is what holds a flood back at the pty.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;

use tauri::ipc::{Channel, InvokeResponseBody};
use tungstenite::{Message, WebSocket};

use crate::live::{Out, Sink};
use crate::local_server::{same_secret, Door, Place};
use crate::remote_http::Request;

pub(crate) mod kind {
    /// Server → page: what the pane printed.
    pub const OUTPUT: u8 = 0;
    /// Page → server: bytes parsed (u32).
    pub const ACK: u8 = 1;
    /// Page → server: typing.
    pub const INPUT: u8 = 2;
    /// Page → server: `rows u16, cols u16`; server → page: the size applied,
    /// or nothing when it was refused.
    pub const RESIZE: u8 = 3;
    /// Page → server: `{ projectId, paneId, rows, cols }`.
    pub const ATTACH: u8 = 4;
    /// Page → server: let the pane go.
    pub const DETACH: u8 = 5;
    /// Server → page: the pane ended; why, or nothing for a plain exit.
    pub const END: u8 = 6;
    /// Server → page: what the pane printed before this attach.
    pub const REPLAY: u8 = 7;
    /// Page → server: the page before what it holds; server → page:
    /// `left u64`, then the bytes.
    pub const OLDER: u8 = 8;
}

/// The longest frame a page sends: a paste.
const MOST_MESSAGE: usize = 8 * 1024 * 1024;

pub(crate) fn encode(handle: u32, kind: u8, bytes: &[u8]) -> Vec<u8> {
    let mut wire = Vec::with_capacity(5 + bytes.len());
    wire.extend_from_slice(&handle.to_le_bytes());
    wire.push(kind);
    wire.extend_from_slice(bytes);
    wire
}

pub(crate) fn decode(wire: &[u8]) -> Option<(u32, u8, &[u8])> {
    let handle = u32::from_le_bytes(wire.get(..4)?.try_into().ok()?);
    Some((handle, *wire.get(4)?, &wire[5..]))
}

/// The desktop window's way in: its ticket, on this server's own host. It is
/// checked before the `Origin` (the window's is `tauri://localhost`, or
/// Vite's in development) because it is the credential, and only the
/// window's IPC hands it out. `None` for anything else, which takes the door.
pub(crate) fn ticket_door(
    request: &Request,
    query: &str,
    place: &Place,
    ticket: &str,
) -> Option<Door> {
    if request.path != "/panes" {
        return None;
    }
    let said = query.split('&').find_map(|pair| pair.strip_prefix("k="))?;
    if !crate::local_server::host_allowed(request.header("host"), place) {
        return Some(Door::Foreign);
    }
    Some(if !said.is_empty() && same_secret(said, ticket) {
        Door::In
    } else {
        Door::Unauthorized
    })
}

/// What a page asked to attach.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Asked {
    pub project_id: String,
    pub pane_id: String,
    pub rows: u16,
    pub cols: u16,
}

/// Said once an attach ends: why, or `None` when its pane exited.
pub(crate) type Ended = Box<dyn FnOnce(Option<String>) + Send>;

/// What the socket does to panes; [`AppPanes`] in the app.
pub(crate) trait Panes: Send + Sync + 'static {
    fn attach(&self, label: &str, client_id: String, asked: Asked, sink: Arc<dyn Sink>, end: Ended);
    fn detach(&self, pane_id: &str, client_id: &str);
    fn input(&self, pane_id: &str, bytes: &[u8]);
    fn resize(&self, pane_id: &str, rows: u16, cols: u16) -> Option<(u16, u16)>;
    fn ack(&self, pane_id: &str, bytes: u32);
    /// Asks for the page before what the attach holds, answered on its
    /// stream. False where there is none to page.
    fn older(&self, pane_id: &str) -> bool;
}

/// One attach's frames, onto the socket's writer.
struct Framed {
    handle: u32,
    out: Sender<Vec<u8>>,
    /// The page went: an attach still running stops at its next frame.
    gone: Arc<AtomicBool>,
}

impl Sink for Framed {
    fn send(&self, out: Out, bytes: Vec<u8>) -> bool {
        let kind = match out {
            Out::Output => kind::OUTPUT,
            Out::Replay => kind::REPLAY,
            Out::Older => kind::OLDER,
        };
        !self.gone.load(Ordering::Acquire)
            && self.out.send(encode(self.handle, kind, &bytes)).is_ok()
    }

    fn acks(&self) -> bool {
        true
    }
}

static SOCKETS: AtomicU64 = AtomicU64::new(1);

/// Upgrades the request and serves the socket until the page goes.
pub(crate) fn serve(mut stream: TcpStream, request: &Request, panes: Arc<dyn Panes>) {
    if !crate::local_events::handshake(&mut stream, request) {
        return;
    }
    // An echo is a few bytes: sent now, not when the kernel has more.
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(None);
    let Ok(writing) = stream.try_clone() else {
        return;
    };
    let config = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(MOST_MESSAGE))
        .max_frame_size(Some(MOST_MESSAGE));
    let label = format!("panes:{}", SOCKETS.fetch_add(1, Ordering::Relaxed));
    let (out, queued) = mpsc::channel::<Vec<u8>>();
    let writer = std::thread::spawn(move || write_all(writing, &queued));
    let reading = WebSocket::from_raw_socket(
        ReadOnly(stream),
        tungstenite::protocol::Role::Server,
        Some(config),
    );
    let gone = Arc::new(AtomicBool::new(false));
    let held = read_all(reading, &label, &out, &gone, panes.as_ref());
    gone.store(true, Ordering::Release);
    for (pane_id, client_id) in held.values() {
        panes.detach(pane_id, client_id);
    }
    drop(out);
    let _ = writer.join();
}

/// The page's frames, until it goes. Answers what it still held.
fn read_all(
    mut ws: WebSocket<ReadOnly>,
    label: &str,
    out: &Sender<Vec<u8>>,
    gone: &Arc<AtomicBool>,
    panes: &dyn Panes,
) -> HashMap<u32, (String, String)> {
    let mut held: HashMap<u32, (String, String)> = HashMap::new();
    loop {
        let wire = match ws.read() {
            Ok(Message::Binary(wire)) => wire,
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(_) => continue,
        };
        let Some((handle, kind, bytes)) = decode(&wire) else {
            continue;
        };
        if kind == kind::ATTACH {
            let Ok(asked) = serde_json::from_slice::<Asked>(bytes) else {
                let _ = out.send(encode(handle, kind::END, b"an attach without its pane"));
                continue;
            };
            let client_id = format!("{label}:{handle}");
            if let Some((pane_id, client_id)) =
                held.insert(handle, (asked.pane_id.clone(), client_id.clone()))
            {
                panes.detach(&pane_id, &client_id);
            }
            let sink: Arc<dyn Sink> = Arc::new(Framed {
                handle,
                out: out.clone(),
                gone: Arc::clone(gone),
            });
            let ending = out.clone();
            let end: Ended = Box::new(move |why| {
                let why = why.unwrap_or_default();
                let _ = ending.send(encode(handle, kind::END, why.as_bytes()));
            });
            panes.attach(label, client_id, asked, sink, end);
            continue;
        }
        if kind == kind::DETACH {
            if let Some((pane_id, client_id)) = held.remove(&handle) {
                panes.detach(&pane_id, &client_id);
            }
            continue;
        }
        let Some((pane_id, _)) = held.get(&handle) else {
            continue;
        };
        match kind {
            kind::INPUT => panes.input(pane_id, bytes),
            kind::ACK => {
                if let Some(acked) = bytes.get(..4) {
                    panes.ack(
                        pane_id,
                        u32::from_le_bytes([acked[0], acked[1], acked[2], acked[3]]),
                    );
                }
            }
            kind::RESIZE => {
                let applied = bytes.get(..4).and_then(|b| {
                    let rows = u16::from_le_bytes([b[0], b[1]]);
                    let cols = u16::from_le_bytes([b[2], b[3]]);
                    panes.resize(pane_id, rows, cols)
                });
                let mut answer = Vec::new();
                if let Some((rows, cols)) = applied {
                    answer.extend_from_slice(&rows.to_le_bytes());
                    answer.extend_from_slice(&cols.to_le_bytes());
                }
                let _ = out.send(encode(handle, kind::RESIZE, &answer));
            }
            kind::OLDER => {
                if !panes.older(pane_id) {
                    let _ = out.send(encode(handle, kind::OLDER, &0u64.to_le_bytes()));
                }
            }
            _ => {}
        }
    }
    held
}

/// Writes what is queued, everything waiting in one flush, until every
/// sender is gone or the page is.
fn write_all(stream: TcpStream, queued: &Receiver<Vec<u8>>) {
    let Ok(closing) = stream.try_clone() else {
        return;
    };
    let mut ws = WebSocket::from_raw_socket(stream, tungstenite::protocol::Role::Server, None);
    let sent = (|| -> tungstenite::Result<()> {
        while let Ok(wire) = queued.recv() {
            ws.write(Message::binary(wire))?;
            while let Ok(more) = queued.try_recv() {
                ws.write(Message::binary(more))?;
            }
            ws.flush()?;
        }
        Ok(())
    })();
    if sent.is_ok() {
        let _ = ws.close(None);
        let _ = ws.flush();
    }
    // The reader is blocked on this socket: this is what lets it go.
    let _ = closing.shutdown(std::net::Shutdown::Both);
}

/// The reading half: what tungstenite would write back from here (a pong, a
/// close) is dropped, so only the writer ever writes to the socket.
struct ReadOnly(TcpStream);

impl Read for ReadOnly {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for ReadOnly {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The panes socket's URL for the desktop window, its ticket and all; `None`
/// when the loopback server is not running, and the window falls back to
/// [`crate::panes::session_attach`]. Hand-written on the page, like the attach it replaces.
#[tauri::command]
pub fn panes_socket() -> Option<String> {
    crate::local_server::panes_url()
}

/// Tauri's channel as an attach's sink: the fallback when the panes socket
/// is not there. Nothing on it acknowledges.
pub(crate) struct ChannelSink(pub Channel<InvokeResponseBody>);

impl Sink for ChannelSink {
    fn send(&self, out: Out, bytes: Vec<u8>) -> bool {
        match out {
            Out::Output | Out::Replay => self.0.send(InvokeResponseBody::Raw(bytes)).is_ok(),
            Out::Older => true,
        }
    }

    fn acks(&self) -> bool {
        false
    }
}

/// The app's panes, through the same claims the window's attach uses.
pub(crate) struct AppPanes(pub tauri::AppHandle);

impl AppPanes {
    fn live(&self, pane_id: &str) -> Option<Arc<crate::claims::Live>> {
        let state = tauri::Manager::state::<crate::sessions::SessionState>(&self.0);
        state.claims.live(pane_id).ok()
    }
}

impl Panes for AppPanes {
    fn attach(
        &self,
        label: &str,
        client_id: String,
        asked: Asked,
        sink: Arc<dyn Sink>,
        end: Ended,
    ) {
        let app = self.0.clone();
        let label = label.to_owned();
        tauri::async_runtime::spawn(async move {
            let ended = crate::panes::attach_with(
                &app,
                label,
                asked.project_id,
                asked.pane_id,
                client_id,
                (asked.rows, asked.cols),
                sink,
            )
            .await;
            end(ended.err().map(|err| err.message));
        });
    }

    fn detach(&self, pane_id: &str, client_id: &str) {
        let state = tauri::Manager::state::<crate::sessions::SessionState>(&self.0);
        let _ = crate::panes::session_detach_now(state, pane_id.to_owned(), client_id.to_owned());
    }

    fn input(&self, pane_id: &str, bytes: &[u8]) {
        if let Some(live) = self.live(pane_id) {
            let _ = live.write(bytes);
        }
    }

    fn resize(&self, pane_id: &str, rows: u16, cols: u16) -> Option<(u16, u16)> {
        let applied = self.live(pane_id)?.resize(rows, cols).ok()?;
        Some((applied.rows, applied.cols))
    }

    fn ack(&self, pane_id: &str, bytes: u32) {
        if let Some(live) = self.live(pane_id) {
            live.ack(bytes);
        }
    }

    fn older(&self, pane_id: &str) -> bool {
        self.live(pane_id).is_some_and(|live| live.ask_older())
    }
}

#[cfg(test)]
#[path = "pane_socket_tests.rs"]
mod tests;
