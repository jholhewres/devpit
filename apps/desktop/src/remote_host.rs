//! The remote viewer's server: the page, the pairing, and the WebSocket.
//!
//! **Never on every interface.** With HTTPS on in the tailnet it listens on
//! loopback alone, and `tailscale serve` carries it to the tailnet with a
//! certificate and the caller's login in `Tailscale-User-Login`. Without
//! HTTPS it listens on the machine's tailnet address alone — tailnet-only —
//! and `tailscale whois` names the caller. Either way the caller must be
//! the machine's own owner in the tailnet *and* a paired device: the tailnet
//! alone is not enough, and neither is the login.

use std::io::BufReader;
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::remote_http::{kind_of, open_one, read, reply, Deadline, Request};

/// What is running, to stop it.
struct Running {
    stop: Arc<AtomicBool>,
    served: bool,
    pub address: String,
}

static RUNNING: Mutex<Option<Running>> = Mutex::new(None);

/// How long a request may take to arrive, its small body and all.
const REQUEST_WITHIN: Duration = Duration::from_secs(10);
/// Connections served at once: each paired viewer keeps one socket open.
const MOST_OPEN: usize = 256;

/// Where the viewer is reached, when the host is up.
pub(crate) fn address() -> Option<String> {
    RUNNING
        .lock()
        .ok()?
        .as_ref()
        .map(|running| running.address.clone())
}

/// Starts the host, or says why it cannot. Started already is fine.
pub(crate) fn start(
    app: &tauri::AppHandle,
    root: PathBuf,
    port: u16,
) -> Result<(String, u16), String> {
    if let Some(running) = RUNNING
        .lock()
        .map_err(|_| "the remote host is stuck")?
        .as_ref()
    {
        return Ok((running.address.clone(), port));
    }
    let tailscale = crate::remote_tailscale::state();
    if !tailscale.installed {
        return Err("Tailscale is not installed on this machine".to_owned());
    }
    if !tailscale.running {
        return Err("Tailscale is not connected — log in to it first".to_owned());
    }
    let owner = tailscale
        .login
        .clone()
        .ok_or("Tailscale does not say who this machine belongs to")?;
    let loopback = TcpListener::bind(("127.0.0.1", port))
        .or_else(|_| TcpListener::bind(("127.0.0.1", 0)))
        .map_err(|err| err.to_string())?;
    let port = loopback.local_addr().map_err(|err| err.to_string())?.port();
    let stop = Arc::new(AtomicBool::new(false));
    let (address, served) = match (&tailscale.name, tailscale.https) {
        (Some(name), true) => {
            crate::remote_tailscale::serve(port)?;
            (
                format!("https://{name}:{}/", crate::remote_tailscale::HTTPS_PORT),
                true,
            )
        }
        _ => {
            let ip: IpAddr = tailscale
                .ip
                .as_deref()
                .and_then(|ip| ip.parse().ok())
                .ok_or("Tailscale has no address for this machine")?;
            // tailnet-only: the machine's tailnet address, never 0.0.0.0.
            let tailnet = TcpListener::bind(SocketAddr::new(ip, port)) // tailnet-only
                .map_err(|err| err.to_string())?;
            accept(
                app.clone(),
                tailnet,
                root.clone(),
                owner.clone(),
                false,
                stop.clone(),
            );
            (format!("http://{ip}:{port}/"), false)
        }
    };
    accept(app.clone(), loopback, root, owner, true, stop.clone());
    *RUNNING.lock().map_err(|_| "the remote host is stuck")? = Some(Running {
        stop,
        served,
        address: address.clone(),
    });
    Ok((address, port))
}

pub(crate) fn stop(app: &tauri::AppHandle) {
    let was = RUNNING.lock().ok().and_then(|mut running| running.take());
    if let Some(running) = was {
        running.stop.store(true, Ordering::SeqCst);
        if running.served {
            crate::remote_tailscale::unserve();
        }
    }
    crate::remote_pairing::withdraw();
    crate::remote_hub::drop_all(app);
}

fn accept(
    app: tauri::AppHandle,
    listener: TcpListener,
    root: PathBuf,
    owner: String,
    loopback: bool,
    stop: Arc<AtomicBool>,
) {
    let serving =
        move |stream, peer| serve(app.clone(), stream, peer, root.clone(), &owner, loopback);
    taking(listener, stop, MOST_OPEN, serving);
}

/// Accepts on `listener` until `stop`, each connection served on a thread of
/// its own, up to `most` at once.
fn taking(
    listener: TcpListener,
    stop: Arc<AtomicBool>,
    most: usize,
    serving: impl Fn(TcpStream, SocketAddr) + Clone + Send + 'static,
) {
    let _ = listener.set_nonblocking(true);
    let open = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        while !stop.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer)) => {
                    // Past the most, it is closed unserved.
                    let Some(opened) = open_one(&open, most) else {
                        continue;
                    };
                    let serving = serving.clone();
                    let _ = std::thread::Builder::new()
                        .name("devpit-remote".into())
                        .spawn(move || {
                            let _opened = opened;
                            serving(stream, peer);
                        });
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(100))
                }
                Err(_) => std::thread::sleep(Duration::from_millis(250)),
            }
        }
    });
}

/// Who called, as the tailnet says: the header `tailscale serve` adds for a
/// connection it carried to loopback, `whois` for one on the tailnet address.
fn caller(request: &Request, peer: SocketAddr, loopback: bool) -> Option<String> {
    if loopback {
        return request.header("tailscale-user-login").map(str::to_owned);
    }
    crate::remote_tailscale::whois(&peer.to_string())
}

/// The request on `stream`, whole within `within`: a timeout per read would
/// let one sent a byte at a time hold its thread for good.
fn request_of(stream: &TcpStream, within: Duration) -> Option<Request> {
    read(BufReader::new(Deadline::new(
        stream.try_clone().ok()?,
        within,
    )))
}

fn serve(
    app: tauri::AppHandle,
    mut stream: TcpStream,
    peer: SocketAddr,
    root: PathBuf,
    owner: &str,
    loopback: bool,
) {
    let _ = stream.set_nonblocking(false);
    let Some(request) = request_of(&stream, REQUEST_WITHIN) else {
        reply(&mut stream, "400 Bad Request", "text/plain", b"");
        return;
    };
    if caller(&request, peer, loopback).as_deref() != Some(owner) {
        reply(
            &mut stream,
            "403 Forbidden",
            "text/plain",
            b"only this machine's owner in the tailnet",
        );
        return;
    }
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/api/ws") => upgrade(app, stream, &request, root),
        ("POST", "/api/pair") => pair(&app, &mut stream, &request, &root),
        ("GET", path) => file(&app, &mut stream, path),
        _ => reply(&mut stream, "404 Not Found", "text/plain", b""),
    }
}

/// The viewer's own files, and nothing else the bundle holds.
fn file(app: &tauri::AppHandle, stream: &mut TcpStream, path: &str) {
    let wanted = match path {
        "/" | "/remote" | "/remote.html" => "remote.html".to_owned(),
        _ => path.trim_start_matches('/').to_owned(),
    };
    let allowed = wanted == "remote.html"
        || wanted == "favicon.png"
        || (wanted.starts_with("assets/") && !wanted.contains(".."));
    let asset = allowed
        .then(|| app.asset_resolver().get(wanted.clone()))
        .flatten();
    match asset {
        Some(asset) => reply(stream, "200 OK", kind_of(&wanted), &asset.bytes),
        None => reply(stream, "404 Not Found", "text/plain", b""),
    }
}

#[derive(serde::Deserialize)]
struct Pairing {
    code: String,
    name: String,
}

/// Why a pairing over the tailnet is refused, if it is: its code must be the
/// one on the machine's screen, and then the person must allow the device.
fn refused(
    asked: &Pairing,
    take: impl FnOnce(&str) -> bool,
    ask: impl FnOnce(&str) -> bool,
) -> Option<&'static [u8]> {
    if !take(&asked.code) {
        return Some(br#"{"error":"that code is not the one on the machine's screen, or it ran out - open a new one"}"#);
    }
    let who = crate::remote_relay::Who::Asking {
        name: asked.name.clone(),
        mcp: false,
    };
    if !crate::remote_relay::allowed_for(&who, ask) {
        return Some(br#"{"error":"the person at the machine did not allow this device"}"#);
    }
    None
}

fn pair(app: &tauri::AppHandle, stream: &mut TcpStream, request: &Request, root: &std::path::Path) {
    let Ok(asked) = serde_json::from_slice::<Pairing>(&request.body) else {
        reply(
            stream,
            "400 Bad Request",
            "application/json",
            br#"{"error":"that is not a pairing"}"#,
        );
        return;
    };
    let refusal = refused(
        &asked,
        |code| crate::remote_pairing::take(code, crate::remote::now()),
        |name| crate::remote_approval::allowed(app, name),
    );
    if let Some(why) = refusal {
        reply(stream, "403 Forbidden", "application/json", why);
        return;
    }
    let Some(token) = crate::remote_devices::fresh_token() else {
        reply(
            stream,
            "500 Internal Server Error",
            "application/json",
            br#"{"error":"no token could be made"}"#,
        );
        return;
    };
    let mut devices = crate::remote_devices::Devices::read(root);
    let device = match devices.pair(&asked.name, &token, crate::remote::now()) {
        Ok(device) => device,
        Err(why) => {
            let body = serde_json::json!({ "error": why }).to_string();
            reply(stream, "409 Conflict", "application/json", body.as_bytes());
            return;
        }
    };
    if devices.write(root).is_err() {
        reply(
            stream,
            "500 Internal Server Error",
            "application/json",
            br#"{"error":"the device could not be kept"}"#,
        );
        return;
    }
    crate::remote::note(root, &device, "paired");
    crate::remote::changed(app);
    let body = serde_json::json!({ "token": token, "device": device.name }).to_string();
    reply(stream, "200 OK", "application/json", body.as_bytes());
}

/// The WebSocket, from this page alone: a page on another origin carrying
/// the person's session along is refused before anything is read.
fn upgrade(app: tauri::AppHandle, mut stream: TcpStream, request: &Request, root: PathBuf) {
    let host = request.header("host").unwrap_or_default();
    let same = request.header("origin").is_some_and(|origin| {
        origin == format!("https://{host}") || origin == format!("http://{host}")
    });
    let (Some(key), true, true) = (
        request.header("sec-websocket-key"),
        same,
        request
            .header("upgrade")
            .is_some_and(|word| word.eq_ignore_ascii_case("websocket")),
    ) else {
        reply(&mut stream, "403 Forbidden", "text/plain", b"");
        return;
    };
    let accept = tungstenite::handshake::derive_accept_key(key.as_bytes());
    let head = format!(
        "HTTP/1.1 101 Switching Protocols\r\nupgrade: websocket\r\nconnection: Upgrade\r\nsec-websocket-accept: {accept}\r\n\r\n"
    );
    if std::io::Write::write_all(&mut stream, head.as_bytes()).is_err() {
        return;
    }
    let config = tungstenite::protocol::WebSocketConfig::default()
        .max_message_size(Some(256 * 1024))
        .max_frame_size(Some(256 * 1024));
    let ws = tungstenite::WebSocket::from_raw_socket(
        stream,
        tungstenite::protocol::Role::Server,
        Some(config),
    );
    crate::remote_conn::run(app, ws, root);
}

#[cfg(test)]
#[path = "remote_host_tests.rs"]
mod tests;
