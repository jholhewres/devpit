use std::collections::HashMap;

use super::*;

const PORT: u16 = 41234;
const TOKEN: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const SESSION: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

fn asked(method: &str, path: &str, headers: &[(&str, &str)]) -> Request {
    Request {
        method: method.to_owned(),
        path: path.to_owned(),
        headers: headers
            .iter()
            .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
            .collect::<HashMap<_, _>>(),
        body: Vec::new(),
    }
}

fn signed_in(path: &str) -> Request {
    let cookie = format!("theme=dark; {COOKIE}={SESSION}");
    asked(
        "POST",
        path,
        &[("host", "127.0.0.1:41234"), ("cookie", &cookie)],
    )
}

#[test]
fn without_the_cookie_or_with_a_wrong_one_the_door_says_401() {
    let bare = asked("POST", "/rpc/project_list", &[("host", "127.0.0.1:41234")]);
    assert_eq!(door(&bare, "", PORT, TOKEN, SESSION), Door::Unauthorized);
    let wrong = asked(
        "POST",
        "/rpc/project_list",
        &[("host", "127.0.0.1:41234"), ("cookie", "devpit_local=nope")],
    );
    assert_eq!(door(&wrong, "", PORT, TOKEN, SESSION), Door::Unauthorized);
    assert_eq!(
        door(&signed_in("/rpc/project_list"), "", PORT, TOKEN, SESSION),
        Door::In
    );
    let token_as_cookie = asked(
        "POST",
        "/rpc/project_list",
        &[
            ("host", "127.0.0.1:41234"),
            ("cookie", &format!("{COOKIE}={TOKEN}")),
        ],
    );
    assert_eq!(
        door(&token_as_cookie, "", PORT, TOKEN, SESSION),
        Door::Unauthorized,
        "the cookie is this run's session, never the token in the home"
    );
}

/// DNS rebinding: a page on another name that resolves to 127.0.0.1 sends
/// its own name as the Host, or its own origin; both are refused before the
/// cookie is even looked at.
#[test]
fn a_foreign_host_or_origin_is_refused_whatever_it_carries() {
    let cookie = format!("{COOKIE}={SESSION}");
    for (host, origin) in [
        ("evil.example:41234", None),
        ("127.0.0.1:9999", None),
        ("127.0.0.1", None),
        ("127.0.0.1:41234", Some("http://evil.example")),
        ("127.0.0.1:41234", Some("http://127.0.0.1:9999")),
        ("localhost:41234", Some("https://localhost:41234")),
        (
            "devpit.localhost:41234",
            Some("http://devpit.localhost:9999"),
        ),
    ] {
        let mut headers = vec![("host", host), ("cookie", cookie.as_str())];
        if let Some(origin) = origin {
            headers.push(("origin", origin));
        }
        let request = asked("POST", "/rpc/project_list", &headers);
        assert_eq!(
            door(&request, "", PORT, TOKEN, SESSION),
            Door::Foreign,
            "{host} {origin:?}"
        );
    }
    let named = asked(
        "GET",
        "/events",
        &[
            ("host", "localhost:41234"),
            ("origin", "http://localhost:41234"),
            ("cookie", cookie.as_str()),
        ],
    );
    assert_eq!(door(&named, "", PORT, TOKEN, SESSION), Door::In);
}

/// The token is traded only under devpit's own name, so the cookie is kept
/// for that name and not for 127.0.0.1, which every loopback server shares.
#[test]
fn the_token_is_traded_for_the_cookie_once_and_only_when_it_is_right() {
    for host in ["127.0.0.1:41234", "localhost:41234"] {
        let elsewhere = asked("GET", "/auth", &[("host", host)]);
        assert_eq!(
            door(&elsewhere, &format!("t={TOKEN}"), PORT, TOKEN, SESSION),
            Door::Unauthorized,
            "{host}"
        );
    }
    let auth = asked("GET", "/auth", &[("host", "devpit.localhost:41234")]);
    assert_eq!(
        door(&auth, &format!("t={TOKEN}"), PORT, TOKEN, SESSION),
        Door::Admit
    );
    assert_eq!(
        door(&auth, "t=wrong", PORT, TOKEN, SESSION),
        Door::Unauthorized
    );
    assert_eq!(door(&auth, "", PORT, TOKEN, SESSION), Door::Unauthorized);
    assert_eq!(door(&auth, "x=1", PORT, TOKEN, SESSION), Door::Unauthorized);
}

#[test]
fn a_stream_or_the_desktops_own_ui_is_not_run_from_a_browser() {
    for command in [
        "chat_send",
        "chat_rejoin",
        "session_attach",
        "browser_open",
        "island_open_chat",
        "tray_read",
        "path_reveal",
        "url_open",
        "plugin:dialog|open",
    ] {
        assert!(desktop_only(command), "{command}");
    }
    for command in ["project_list", "chat_history", "board_get", "chat_head"] {
        assert!(!desktop_only(command), "{command}");
    }
}

#[cfg(unix)]
#[test]
fn the_token_is_made_once_owner_only_and_kept() {
    use std::os::unix::fs::PermissionsExt;
    let home = tempfile::tempdir().expect("home");
    let first = token_in(home.path()).expect("made");
    assert_eq!(first.len(), 64);
    let mode = std::fs::metadata(home.path().join(TOKEN_FILE))
        .expect("kept")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    assert_eq!(token_in(home.path()).expect("read"), first);
}

/// Nothing listens off loopback: the socket is bound to 127.0.0.1, and the
/// port is kept for next time.
#[test]
fn the_server_binds_loopback_alone_and_keeps_its_port() {
    let home = tempfile::tempdir().expect("home");
    let listener = bound(home.path()).expect("bound");
    let address = listener.local_addr().expect("address");
    assert!(address.ip().is_loopback(), "{address}");
    let kept: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(home.path().join(PORT_FILE)).expect("port kept"),
    )
    .expect("json");
    assert_eq!(kept["port"], address.port());
    drop(listener);
    assert_eq!(
        bound(home.path())
            .expect("again")
            .local_addr()
            .expect("address")
            .port(),
        address.port()
    );
}

#[test]
fn the_sign_in_link_goes_to_loopback() {
    assert_eq!(
        sign_in_url(PORT, "t0k"),
        "http://devpit.localhost:41234/auth?t=t0k"
    );
}

#[test]
fn a_chromium_opens_it_as_an_app_and_otherwise_the_desktop_opens_the_link() {
    assert_eq!(
        opener(|name| name == "google-chrome"),
        ("google-chrome", true)
    );
    assert_eq!(opener(|name| name == "chromium"), ("chromium", true));
    assert_eq!(opener(|_| false), ("xdg-open", false));
}

/// A pasted picture or a saved file is a call of megabytes: it is read whole,
/// and only one past the ceiling is refused.
#[test]
fn a_call_of_megabytes_is_read_and_one_past_the_ceiling_is_not() {
    let call = |length: usize| {
        let body = "x".repeat(length);
        format!(
            "POST /rpc/file_write HTTP/1.1\r\nhost: 127.0.0.1:41234\r\ncontent-length: {length}\r\n\r\n{body}"
        )
    };
    let big = call(2 * 1024 * 1024);
    let (request, _) = read_with_query(&mut big.as_bytes()).expect("read");
    assert_eq!(request.body.len(), 2 * 1024 * 1024);
    let over = call(MOST_BODY + 1);
    assert!(read_with_query(&mut over.as_bytes()).is_none());
}

#[test]
fn each_run_signs_in_with_a_new_session() {
    let (one, two) = (secret().expect("one"), secret().expect("two"));
    assert_eq!(one.len(), 64);
    assert_ne!(one, two);
}
