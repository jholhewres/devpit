//! devpit on loopback: the same app in a browser at `127.0.0.1:<port>`.
//!
//! **Loopback alone, and a cookie at the door.** It never binds anything but
//! `127.0.0.1`. A browser earns the cookie once, from a token written
//! owner-only in the home (`/auth?t=…`, then a redirect that takes the token
//! out of the URL). Every request and every WebSocket upgrade must name this
//! port on `127.0.0.1` or `localhost` as its `Host`, and a same-port `Origin`
//! when it has one: a page elsewhere that resolves its own name to 127.0.0.1
//! (DNS rebinding) is refused before anything runs.
//!
//! **One way in for every command.** `POST /rpc/<command>` hands the call to
//! the main webview's own invoke handler, as the window's IPC does, so a
//! command answers the same whichever door it came through. The invoke key
//! that handler asks for never leaves this process: the browser holds only
//! the cookie. A command that streams over a channel, or that opens UI on the
//! desktop, answers 501 here: a browser must not make the desktop's window
//! open a dialog nobody in front of it asked for.

use std::io::{BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use crate::remote_http::{read_up_to, reply, Request};

/// The port, in the home, so "open in Chrome" and a restart find it.
pub(crate) const PORT_FILE: &str = "local.json";
/// The token a browser trades for its cookie, owner-only in the home.
pub(crate) const TOKEN_FILE: &str = "local-token";
const COOKIE: &str = "devpit_local";
/// The name the browser is sent to. A cookie is kept per host, not per port,
/// so on `127.0.0.1` every other loopback server would be sent it too; a
/// `*.localhost` name resolves to loopback and is this server's alone.
pub(crate) const HOST: &str = "devpit.localhost";
/// The largest call a browser sends: a pasted picture (8 MiB, as base64) or a
/// saved file, with room for the JSON around it.
const MOST_BODY: usize = 16 * 1024 * 1024;

/// Commands a browser may not run here: each streams over a Tauri channel
/// (the browser gets that stream over the events socket), or opens UI on the
/// desktop's screen.
const DESKTOP_ONLY: &[&str] = &[
    "chat_send",
    "chat_rejoin",
    "session_attach",
    "chat_transcribe",
    "path_open",
    "path_reveal",
    "url_open",
];
/// Whole families of desktop-only commands, by prefix.
const DESKTOP_ONLY_FAMILIES: &[&str] = &["browser_", "island_", "tray_", "plugin:"];

pub(crate) fn desktop_only(command: &str) -> bool {
    DESKTOP_ONLY.contains(&command)
        || DESKTOP_ONLY_FAMILIES
            .iter()
            .any(|family| command.starts_with(family))
}

/// The hosts this server answers for, on its own port.
fn ours(host: &str, port: u16) -> bool {
    ["127.0.0.1", "localhost", HOST]
        .iter()
        .any(|name| host == format!("{name}:{port}"))
}

/// The `Host` a request must carry: this port on loopback, by number or name.
pub(crate) fn host_allowed(host: Option<&str>, port: u16) -> bool {
    host.is_some_and(|host| ours(host, port))
}

/// No `Origin` (a top-level navigation, curl) or this very origin.
pub(crate) fn origin_allowed(origin: Option<&str>, port: u16) -> bool {
    match origin {
        None => true,
        Some(origin) => origin
            .strip_prefix("http://")
            .is_some_and(|host| ours(host, port)),
    }
}

/// The cookie's value in a `Cookie` header, if it is there.
pub(crate) fn cookie_of(header: Option<&str>) -> Option<&str> {
    header?
        .split(';')
        .map(str::trim)
        .find_map(|pair| pair.strip_prefix(&format!("{COOKIE}=")))
}

/// Two secrets compared without leaking where they first differ.
pub(crate) fn same_secret(said: &str, kept: &str) -> bool {
    said.len() == kept.len()
        && said
            .bytes()
            .zip(kept.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}

/// The token in `/auth?t=…`, from the request's first line.
fn token_in_query(query: &str) -> Option<&str> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix("t="))
        .filter(|token| !token.is_empty())
}

/// What one request gets, decided before anything runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Door {
    /// Host or Origin is not this server's.
    Foreign,
    /// A right token: set the cookie and send the browser on without it.
    Admit,
    /// No cookie, or the wrong one.
    Unauthorized,
    /// Past the door.
    In,
}

/// `token` is the one in the home, traded once per browser at `/auth`;
/// `session` is the cookie it is traded for, made for this run of the app.
pub(crate) fn door(request: &Request, query: &str, port: u16, token: &str, session: &str) -> Door {
    if !host_allowed(request.header("host"), port)
        || !origin_allowed(request.header("origin"), port)
    {
        return Door::Foreign;
    }
    if request.method == "GET" && request.path == "/auth" {
        /* Signed in only under its own name, so the cookie is never kept for
        127.0.0.1, where every other loopback server would be sent it. */
        if request.header("host") != Some(&format!("{HOST}:{port}")) {
            return Door::Unauthorized;
        }
        return match token_in_query(query) {
            Some(said) if same_secret(said, token) => Door::Admit,
            _ => Door::Unauthorized,
        };
    }
    match cookie_of(request.header("cookie")) {
        Some(said) if same_secret(said, session) => Door::In,
        _ => Door::Unauthorized,
    }
}

/// 32 random bytes, as hex.
pub(crate) fn secret() -> std::io::Result<String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|err| std::io::Error::other(err.to_string()))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

/// The token, made once: 32 random bytes as hex, owner-only from its first byte.
pub(crate) fn token_in(home: &Path) -> std::io::Result<String> {
    let path = home.join(TOKEN_FILE);
    if let Ok(kept) = std::fs::read_to_string(&path) {
        let kept = kept.trim();
        if kept.len() == 64 && kept.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(kept.to_owned());
        }
    }
    let token = secret()?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(token.as_bytes())?;
    Ok(token)
}

/// The listener on loopback: the port kept in the home when it is free, else
/// any, written back for next time.
pub(crate) fn bound(home: &Path) -> std::io::Result<TcpListener> {
    let kept: Option<u16> = std::fs::read_to_string(home.join(PORT_FILE))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .and_then(|value| value["port"].as_u64())
        .and_then(|port| u16::try_from(port).ok());
    let listener = kept
        .and_then(|port| TcpListener::bind(("127.0.0.1", port)).ok())
        .map_or_else(|| TcpListener::bind(("127.0.0.1", 0)), Ok)?;
    let port = listener.local_addr()?.port();
    let _ = std::fs::write(
        home.join(PORT_FILE),
        serde_json::json!({ "port": port }).to_string(),
    );
    Ok(listener)
}

/// Where the browser is sent to sign in, token and all: for "open in Chrome".
pub(crate) fn sign_in_url(port: u16, token: &str) -> String {
    format!("http://{HOST}:{port}/auth?t={token}")
}

static PORT: Mutex<Option<u16>> = Mutex::new(None);

/// The port the server is on, once it is.
pub(crate) fn port() -> Option<u16> {
    *PORT.lock().ok()?
}

/// Starts the server on loopback, its door kept by `token`. Each request
/// runs on its own thread; `serve` decides what it is.
pub(crate) fn start(app: tauri::AppHandle, home: &Path) -> std::io::Result<u16> {
    let token = token_in(home)?;
    let session = secret()?;
    let listener = bound(home)?;
    let port = listener.local_addr()?.port();
    if let Ok(mut kept) = PORT.lock() {
        *kept = Some(port);
    }
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let (app, token, session) = (app.clone(), token.clone(), session.clone());
            std::thread::spawn(move || serve(app, stream, port, &token, &session));
        }
    });
    Ok(port)
}

fn serve(app: tauri::AppHandle, mut stream: TcpStream, port: u16, token: &str, session: &str) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let Some(reader) = stream.try_clone().ok() else {
        return;
    };
    let mut reader = BufReader::new(reader);
    let Some((request, query)) = read_with_query(&mut reader) else {
        reply(&mut stream, "400 Bad Request", "text/plain", b"");
        return;
    };
    match door(&request, &query, port, token, session) {
        Door::Foreign => reply(
            &mut stream,
            "403 Forbidden",
            "text/plain",
            b"not this server",
        ),
        Door::Unauthorized => reply(
            &mut stream,
            "401 Unauthorized",
            "text/plain",
            b"sign in from devpit",
        ),
        Door::Admit => admitted(&mut stream, session),
        Door::In => match (request.method.as_str(), request.path.as_str()) {
            ("POST", path) if path.starts_with("/rpc/") => {
                crate::local_rpc::call(&app, &mut stream, &path["/rpc/".len()..], &request.body)
            }
            ("GET", "/events") => crate::local_events::upgrade(stream, &request),
            ("GET", path) => crate::local_files::serve(&app, &mut stream, path),
            _ => reply(&mut stream, "404 Not Found", "text/plain", b""),
        },
    }
}

/// The request and its query string, which [`read_up_to`] drops.
pub(crate) fn read_with_query(reader: &mut impl std::io::BufRead) -> Option<(Request, String)> {
    let first = reader.fill_buf().ok()?;
    let line_end = first
        .iter()
        .position(|b| *b == b'\n')
        .unwrap_or(first.len());
    let line = String::from_utf8_lossy(&first[..line_end]).into_owned();
    let query = line
        .split_whitespace()
        .nth(1)
        .and_then(|target| target.split_once('?'))
        .map(|(_, query)| query.to_owned())
        .unwrap_or_default();
    Some((read_up_to(reader, MOST_BODY)?, query))
}

/// The cookie set, and the browser sent on to the app without the token.
fn admitted(stream: &mut TcpStream, session: &str) {
    let head = format!(
        "HTTP/1.1 303 See Other\r\nlocation: /\r\n\
         set-cookie: {COOKIE}={session}; HttpOnly; SameSite=Strict; Path=/\r\n\
         cache-control: no-store\r\nreferrer-policy: no-referrer\r\n\
         content-length: 0\r\nconnection: close\r\n\r\n"
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.flush();
}

/// The browser that opens the app as an app: Chromium's `--app`, when one is
/// installed, else whatever the desktop opens links with.
pub(crate) fn opener(found: impl Fn(&str) -> bool) -> (&'static str, bool) {
    [
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
    ]
    .into_iter()
    .find(|program| found(program))
    .map_or(("xdg-open", false), |program| (program, true))
}

/// "Open in browser": signs this browser in and opens the app, off the
/// window's thread. The token goes on the command line of a process the
/// person's own user runs, and leaves the URL at the first redirect.
pub(crate) fn open_in_browser() {
    let (Some(port), Ok(home)) = (port(), devpit_core::Store::root()) else {
        return;
    };
    let Ok(token) = token_in(&home) else {
        return;
    };
    std::thread::spawn(move || {
        let url = sign_in_url(port, &token);
        let path = devpit_pty::login_path::search_path();
        let (program, app) = opener(|name| {
            devpit_pty::host_env::found_on(path, std::ffi::OsStr::new(name)).is_some()
        });
        let mut command = devpit_pty::host_env::command(program);
        if app {
            command.arg(format!("--app={url}"));
        } else {
            command.arg(url);
        }
        if let Ok(child) = command.spawn() {
            devpit_pty::host_env::let_go(child);
        }
    });
}

#[cfg(test)]
#[path = "local_server_tests.rs"]
mod tests;
