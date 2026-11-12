//! devpit on loopback: the same app in a browser at `127.0.0.1:<port>`.
//!
//! **Loopback alone, and a cookie at the door.** It binds only `127.0.0.1`.
//! A browser trades a one-time code (`/auth?c=…`), or the token kept
//! owner-only in the home (`/auth?t=…`), for a session cookie made per run,
//! under a name of this install's own (`<random>.localhost`), since a cookie
//! is kept per name and not per port. `Host` and `Origin` must be this port
//! under that name, `127.0.0.1` or `localhost`, which refuses DNS rebinding
//! before anything runs.
//!
//! **One way in for every command.** `POST /rpc/<command>` hands the call to
//! the main webview's own invoke handler, as the window's IPC does, so a
//! command answers the same whichever door it came through. The invoke key
//! that handler asks for never leaves this process: the browser holds only
//! the cookie. A command that streams over a channel, or that opens UI on the
//! desktop, answers 501 here: a browser must not make the desktop's window
//! open a dialog nobody in front of it asked for.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::remote_http::{read_body, read_head, reply, Deadline, Request};

/// The port and the name, owner-only in the home, so "open in Chrome" and a
/// restart find them.
pub(crate) const PORT_FILE: &str = "local.json";
/// The token a browser trades for its cookie, owner-only in the home.
pub(crate) const TOKEN_FILE: &str = "local-token";
const COOKIE: &str = "devpit_local";
/// The largest call a browser sends: a pasted picture (8 MiB, as base64) or a
/// saved file, with room for the JSON around it.
const MOST_BODY: usize = 16 * 1024 * 1024;
/// How long a request's head may take to arrive, and then its body.
const HEAD_WITHIN: Duration = Duration::from_secs(10);
const BODY_WITHIN: Duration = Duration::from_secs(60);
/// Connections served at once: every browser tab keeps an events socket and
/// a panes socket, and anyone on the machine can connect to loopback.
const MOST_OPEN: usize = 256;
/// How long a one-time sign-in code is good for.
const CODE_LIFE: Duration = Duration::from_secs(60);

/// Commands a browser may not run here: each streams over a Tauri channel
/// (the browser gets that stream over the events socket), or opens UI on the
/// desktop's screen.
const DESKTOP_ONLY: &[&str] = &[
    "chat_send",
    "chat_rejoin",
    "session_attach",
    "panes_socket",
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

/// Where this server is: its port, and the name it signs a browser in under.
/// A cookie is kept per name, not per port, so the name is this install's own
/// and random: no other loopback server is ever reached by it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Place {
    pub port: u16,
    pub name: String,
}

impl Place {
    /// `host:port` under the name the cookie is kept for.
    fn signed_in_host(&self) -> String {
        format!("{}:{}", self.name, self.port)
    }

    fn ours(&self, host: &str) -> bool {
        host == self.signed_in_host()
            || ["127.0.0.1", "localhost"]
                .iter()
                .any(|name| host == format!("{name}:{}", self.port))
    }
}

/// The `Host` a request must carry: this port on loopback, by number or name.
pub(crate) fn host_allowed(host: Option<&str>, place: &Place) -> bool {
    host.is_some_and(|host| place.ours(host))
}

/// No `Origin` (a top-level navigation, curl) or this very origin.
pub(crate) fn origin_allowed(origin: Option<&str>, place: &Place) -> bool {
    match origin {
        None => true,
        Some(origin) => origin
            .strip_prefix("http://")
            .is_some_and(|host| place.ours(host)),
    }
}

/// Every value of the cookie in a `Cookie` header: a sibling `.localhost`
/// server can plant one of the same name, so the first is not always ours.
pub(crate) fn cookies_of(header: Option<&str>) -> impl Iterator<Item = &str> {
    header
        .unwrap_or_default()
        .split(';')
        .map(str::trim)
        .filter_map(|pair| pair.strip_prefix(COOKIE)?.strip_prefix('='))
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

/// The value of `name` in a query string, when it has one.
fn in_query<'a>(query: &'a str, name: &str) -> Option<&'a str> {
    query
        .split('&')
        .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
        .filter(|value| !value.is_empty())
}

/// Codes "Open in browser" signs in with, each good once and for
/// [`CODE_LIFE`]: the link sits on Chromium's command line, which every user
/// of the machine can read, so it must not carry the lasting token.
pub(crate) struct Codes(Mutex<Vec<(String, Instant)>>);

impl Codes {
    pub(crate) const fn new() -> Codes {
        Codes(Mutex::new(Vec::new()))
    }

    pub(crate) fn issue(&self, now: Instant) -> std::io::Result<String> {
        let code = secret()?;
        let mut codes = self
            .0
            .lock()
            .map_err(|_| std::io::Error::other("poisoned"))?;
        codes.retain(|(_, until)| *until > now);
        codes.push((code.clone(), now + CODE_LIFE));
        Ok(code)
    }

    /// Whether `said` is a code still good at `now`; it is good no more.
    pub(crate) fn redeem(&self, said: &str, now: Instant) -> bool {
        let Ok(mut codes) = self.0.lock() else {
            return false;
        };
        let found = codes
            .iter()
            .position(|(code, until)| *until > now && same_secret(said, code));
        found.map(|at| codes.remove(at)).is_some()
    }
}

static CODES: Codes = Codes::new();

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

/// What a request is checked against: where this server is, the token in the
/// home and the one-time codes traded at `/auth`, the cookie they are traded
/// for (made for this run of the app) and the desktop window's panes ticket.
pub(crate) struct Keys<'a> {
    pub place: &'a Place,
    pub token: &'a str,
    pub codes: &'a Codes,
    pub session: &'a str,
    pub ticket: &'a str,
}

pub(crate) fn door(request: &Request, keys: &Keys) -> Door {
    let Keys {
        place,
        token,
        codes,
        session,
        ..
    } = *keys;
    if !host_allowed(request.header("host"), place)
        || !origin_allowed(request.header("origin"), place)
    {
        return Door::Foreign;
    }
    if request.method == "GET" && request.path == "/auth" {
        /* Signed in only under this install's own name: a cookie kept for
        127.0.0.1 or a shared name would reach every other server there. */
        if request.header("host") != Some(&place.signed_in_host()) {
            return Door::Unauthorized;
        }
        let query = request.query.as_str();
        let code = in_query(query, "c").is_some_and(|said| codes.redeem(said, Instant::now()));
        let token = in_query(query, "t").is_some_and(|said| same_secret(said, token));
        return if code || token {
            Door::Admit
        } else {
            Door::Unauthorized
        };
    }
    if cookies_of(request.header("cookie")).any(|said| same_secret(said, session)) {
        Door::In
    } else {
        Door::Unauthorized
    }
}

/// 32 random bytes, as hex.
pub(crate) fn secret() -> std::io::Result<String> {
    random_hex::<32>()
}

fn random_hex<const N: usize>() -> std::io::Result<String> {
    let mut bytes = [0u8; N];
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
    owner_only(&path, &token)?;
    Ok(token)
}

/// Writes `text` to `path`, readable by its owner alone from the first byte.
fn owner_only(path: &Path, text: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(text.as_bytes())
}

/// A name this install signs in under: random, so no other server shares it,
/// and under `.localhost`, which a browser resolves to loopback.
fn is_own_name(name: &str) -> bool {
    name.strip_suffix(".devpit.localhost").is_some_and(|label| {
        label.len() == 32
            && label
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    })
}

/// The listener on loopback, and where it is: the port and name kept in the
/// home, the port when it is still free, written back for next time.
pub(crate) fn bound(home: &Path) -> std::io::Result<(TcpListener, Place)> {
    let kept: serde_json::Value = std::fs::read_to_string(home.join(PORT_FILE))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let listener = kept["port"]
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .and_then(|port| TcpListener::bind(("127.0.0.1", port)).ok())
        .map_or_else(|| TcpListener::bind(("127.0.0.1", 0)), Ok)?;
    let name = match kept["name"].as_str().filter(|name| is_own_name(name)) {
        Some(name) => name.to_owned(),
        None => format!("{}.devpit.localhost", random_hex::<16>()?),
    };
    let place = Place {
        port: listener.local_addr()?.port(),
        name,
    };
    /* Owner-only: the name is what keeps the cookie from other servers. */
    let _ = owner_only(
        &home.join(PORT_FILE),
        &serde_json::json!({ "port": place.port, "name": place.name }).to_string(),
    );
    Ok((listener, place))
}

/// Where the browser is sent to sign in with a one-time code: "open in Chrome".
pub(crate) fn sign_in_url(place: &Place, code: &str) -> String {
    format!("http://{}/auth?c={code}", place.signed_in_host())
}

/// A connection served; one fewer open once it is dropped.
pub(crate) struct Opened(Arc<AtomicUsize>);

impl Drop for Opened {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// One more connection, unless `most` are open already.
pub(crate) fn open_one(open: &Arc<AtomicUsize>, most: usize) -> Option<Opened> {
    open.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |now| {
        (now < most).then_some(now + 1)
    })
    .ok()?;
    Some(Opened(Arc::clone(open)))
}

static PLACE: Mutex<Option<Place>> = Mutex::new(None);
/// The desktop window's panes socket, ticket and all, for this run.
static PANES_URL: Mutex<Option<String>> = Mutex::new(None);

/// Where the desktop window opens its panes socket, once the server is up.
pub(crate) fn panes_url() -> Option<String> {
    PANES_URL.lock().ok()?.clone()
}

/// Where the server is, once it is.
pub(crate) fn place() -> Option<Place> {
    PLACE.lock().ok()?.clone()
}

/// Starts the server on loopback, its door kept by `token`. Each request
/// runs on its own thread, up to [`MOST_OPEN`]; `serve` decides what it is.
pub(crate) fn start(app: tauri::AppHandle, home: &Path) -> std::io::Result<u16> {
    let token = token_in(home)?;
    let session = secret()?;
    let ticket = secret()?;
    let (listener, place) = bound(home)?;
    let port = place.port;
    if let Ok(mut kept) = PLACE.lock() {
        *kept = Some(place.clone());
    }
    if let Ok(mut kept) = PANES_URL.lock() {
        *kept = Some(format!("ws://127.0.0.1:{port}/panes?k={ticket}"));
    }
    let open = Arc::new(AtomicUsize::new(0));
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else {
                // Out of descriptors, most likely: spinning on it would not free one.
                std::thread::sleep(Duration::from_millis(100));
                continue;
            };
            let Some(opened) = open_one(&open, MOST_OPEN) else {
                continue;
            };
            let (app, place, token, session, ticket) = (
                app.clone(),
                place.clone(),
                token.clone(),
                session.clone(),
                ticket.clone(),
            );
            let _ = std::thread::Builder::new()
                .name("devpit-local".into())
                .spawn(move || {
                    let _opened = opened;
                    let keys = Keys {
                        place: &place,
                        token: &token,
                        codes: &CODES,
                        session: &session,
                        ticket: &ticket,
                    };
                    serve(app, stream, &keys);
                });
        }
    });
    Ok(port)
}

fn serve(app: tauri::AppHandle, mut stream: TcpStream, keys: &Keys) {
    let Some(reader) = stream.try_clone().ok() else {
        return;
    };
    let mut reader = BufReader::new(Deadline::new(reader, HEAD_WITHIN));
    let Some((entry, request)) = admitted(&mut reader, keys, |reader| {
        reader.get_mut().renew(BODY_WITHIN);
    }) else {
        reply(&mut stream, "400 Bad Request", "text/plain", b"");
        return;
    };
    match entry {
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
        Door::Admit => signed_in(&mut stream, keys.session),
        Door::In => match (request.method.as_str(), request.path.as_str()) {
            ("POST", path) if path.starts_with("/rpc/") => {
                crate::local_rpc::call(&app, &mut stream, &path["/rpc/".len()..], &request.body)
            }
            ("GET", "/events") => crate::local_events::upgrade(stream, &request),
            ("GET", "/panes") => crate::pane_socket::serve(
                stream,
                &request,
                std::sync::Arc::new(crate::pane_socket::AppPanes(app)),
            ),
            ("GET", path) => crate::local_files::serve(&app, &mut stream, path),
            _ => reply(&mut stream, "404 Not Found", "text/plain", b""),
        },
    }
}

/// A request's head and what the door says of it, and its body only for a
/// call past the door: nobody unknown gets megabytes read. `before_body`
/// runs first, to give the body a deadline of its own.
pub(crate) fn admitted<R: BufRead>(
    reader: &mut R,
    keys: &Keys,
    before_body: impl FnOnce(&mut R),
) -> Option<(Door, Request)> {
    let mut request = read_head(&mut *reader)?;
    let entry = crate::pane_socket::ticket_door(&request, &request.query, keys.place, keys.ticket)
        .unwrap_or_else(|| door(&request, keys));
    if entry == Door::In && request.method == "POST" && request.path.starts_with("/rpc/") {
        before_body(reader);
        read_body(&mut *reader, &mut request, MOST_BODY)?;
    }
    Some((entry, request))
}

/// The cookie set, and the browser sent on to the app without the token.
fn signed_in(stream: &mut TcpStream, session: &str) {
    let head = format!(
        "HTTP/1.1 303 See Other\r\nlocation: /\r\n\
         set-cookie: {COOKIE}={session}; HttpOnly; SameSite=Strict; Path=/\r\n\
         cache-control: no-store\r\nreferrer-policy: no-referrer\r\n\
         content-length: 0\r\nconnection: close\r\n\r\n"
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.flush();
}

/// What opens a link on `os` where no Chromium is: the desktop's own opener.
pub(crate) fn link_opener(os: &str) -> (&'static str, &'static [&'static str]) {
    match os {
        "macos" => ("open", &[]),
        "windows" => ("cmd", &["/C", "start", ""]),
        _ => ("xdg-open", &[]),
    }
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
    .map_or((link_opener(std::env::consts::OS).0, false), |program| {
        (program, true)
    })
}

/// "Open in browser": signs this browser in and opens the app, off the
/// window's thread. A one-time code goes on the browser's command line, and
/// leaves the URL at the first redirect.
pub(crate) fn open_in_browser() {
    let Some(place) = place() else {
        return;
    };
    let Ok(code) = CODES.issue(Instant::now()) else {
        return;
    };
    std::thread::spawn(move || {
        let url = sign_in_url(&place, &code);
        let path = devpit_pty::login_path::search_path();
        let (program, app) = opener(|name| {
            devpit_pty::host_env::found_on(path, std::ffi::OsStr::new(name)).is_some()
        });
        let mut command = devpit_pty::host_env::command(program);
        if app {
            command.arg(format!("--app={url}"));
        } else {
            command.args(link_opener(std::env::consts::OS).1).arg(url);
        }
        if let Ok(child) = command.spawn() {
            devpit_pty::host_env::let_go(child);
        }
    });
}

#[cfg(test)]
#[path = "local_server_tests.rs"]
mod tests;
