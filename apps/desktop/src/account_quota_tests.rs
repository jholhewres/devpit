use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use devpit_rpc::{AccountQuota, ClaudeAccount, PlanWindow, QuotaState};

use super::*;

/// A token no other string in a test could contain by accident.
const TOKEN: &str = "sk-ant-oat01-NEVER-LEAVES-THIS-PROCESS-7f3a";

fn window(label: &str, percent: f64) -> PlanWindow {
    PlanWindow {
        label: label.to_owned(),
        percent,
        resets_at: None,
    }
}

/// An account whose `.credentials.json` holds `TOKEN`, expiring at `expires_ms`.
fn account_with(expires_ms: f64) -> (tempfile::TempDir, ClaudeAccount) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join(".credentials.json"),
        format!(
            r#"{{"claudeAiOauth":{{"accessToken":"{TOKEN}","refreshToken":"r-1","expiresAt":{expires_ms}}}}}"#
        ),
    )
    .expect("credentials");
    let account = ClaudeAccount {
        name: "work".to_owned(),
        config_dir: dir.path().display().to_string(),
    };
    (dir, account)
}

fn later_ms() -> f64 {
    (now() + 3600.0) * 1000.0
}

/// Answers one request with `status` and `body`, and hands back what it was sent.
fn serve_once(status: &str, body: &'static str) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!(
        "http://{}/api/oauth/usage",
        listener.local_addr().expect("addr")
    );
    let status = status.to_owned();
    let served = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("timeout");
        let mut request = Vec::new();
        let mut chunk = [0u8; 4096];
        while !request.windows(4).any(|end| end == b"\r\n\r\n") {
            let read = stream.read(&mut chunk).expect("read");
            if read == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..read]);
        }
        let reply = format!(
            "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        stream.write_all(reply.as_bytes()).expect("reply");
        String::from_utf8_lossy(&request).into_owned()
    });
    (url, served)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("client")
}

fn asked(url: &str, account: &ClaudeAccount) -> AccountQuota {
    let memory = Mutex::new(Memory::default());
    tauri::async_runtime::block_on(quota_at(&client(), url, account, &memory))
}

/// Every way the window or a log could see an answer.
fn everything_said(quota: &AccountQuota) -> String {
    format!(
        "{}\n{quota:?}\n{}",
        serde_json::to_string(quota).expect("json"),
        serde_json::to_string(&devpit_rpc::AccountQuotas {
            accounts: vec![quota.clone()]
        })
        .expect("json")
    )
}

#[test]
fn the_token_is_sent_to_the_endpoint_and_never_reaches_the_window() {
    for (status, body, state) in [
        (
            "200 OK",
            r#"{"five_hour":{"utilization":95.0},"seven_day":{"utilization":40.0}}"#,
            QuotaState::Read,
        ),
        (
            "401 Unauthorized",
            r#"{"error":"bad token"}"#,
            QuotaState::Expired,
        ),
        ("429 Too Many Requests", "{}", QuotaState::Unavailable),
        ("500 Internal Server Error", "oops", QuotaState::Unavailable),
    ] {
        let (_dir, account) = account_with(later_ms());
        let (url, served) = serve_once(status, body);
        let quota = asked(&url, &account);
        let request = served.join().expect("served");
        assert!(
            request.contains(&format!("Bearer {TOKEN}")),
            "the endpoint was asked with the saved token ({status})"
        );
        assert_eq!(quota.state, state, "{status}");
        assert!(
            !everything_said(&quota).contains(TOKEN),
            "the token reached the payload ({status})"
        );
    }
}

#[test]
fn an_endpoint_that_cannot_be_reached_says_unavailable_and_nothing_of_the_request() {
    let (_dir, account) = account_with(later_ms());
    // Bound and dropped: nothing listens there now.
    let port = TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port();
    let quota = asked(
        &format!("http://127.0.0.1:{port}/api/oauth/usage"),
        &account,
    );
    assert_eq!(quota.state, QuotaState::Unavailable);
    assert!(!everything_said(&quota).contains(TOKEN));
}

#[test]
fn a_lapsed_token_is_never_sent_and_says_to_open_a_session() {
    let (_dir, account) = account_with(1000.0);
    // No server at all: a request would fail the test by being Unavailable.
    let quota = asked("http://127.0.0.1:9/never", &account);
    assert_eq!(quota.state, QuotaState::Expired);
}

#[test]
fn a_directory_without_a_sign_in_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let account = ClaudeAccount {
        name: "empty".to_owned(),
        config_dir: dir.path().display().to_string(),
    };
    assert_eq!(
        asked("http://127.0.0.1:9/never", &account).state,
        QuotaState::SignedOut
    );
    assert!(matches!(saved_in("not json", 0.0), Saved::SignedOut));
    assert!(matches!(
        saved_in(r#"{"claudeAiOauth":{}}"#, 0.0),
        Saved::SignedOut
    ));
}

#[test]
fn the_four_windows_are_read_and_a_full_five_hour_window_is_nearly_out() {
    let body: serde_json::Value = serde_json::from_str(
        r#"{"five_hour":{"utilization":91},"seven_day":{"utilization":10},
            "seven_day_opus":{"utilization":20},"seven_day_sonnet":{"utilization":30}}"#,
    )
    .expect("json");
    let windows = crate::plan_limits::windows_of(&body);
    assert_eq!(windows.len(), 4);
    assert!(nearly_out(&windows));
    assert!(!nearly_out(&[window("5-hour", 90.0)]), "past 90, not at it");
    assert!(
        !nearly_out(&[window("Weekly", 99.0)]),
        "only the 5-hour window"
    );
}

#[test]
fn a_read_is_kept_a_minute_and_a_429_cools_the_endpoint_for_five() {
    let mut memory = Memory::default();
    let start = Instant::now();
    let read = AccountQuota {
        name: "work".to_owned(),
        config_dir: "/w".to_owned(),
        state: QuotaState::Read,
        windows: vec![window("5-hour", 50.0)],
        nearly_out: false,
        read_at: 1.0,
    };
    memory.heard("/w", &read, false, start);
    assert_eq!(
        memory.kept("/w", start + Duration::from_secs(59)),
        Some(read.clone())
    );
    assert_eq!(memory.kept("/w", start + FRESH_FOR), None);

    let refused = start + FRESH_FOR;
    memory.heard("/w", &read, true, refused);
    let cooling = memory
        .kept("/w", refused + COOL_DOWN - Duration::from_secs(1))
        .expect("cooling down");
    assert_eq!(cooling.state, QuotaState::Unavailable);
    assert_eq!(cooling.windows, read.windows, "the last read stays drawn");
    assert_eq!(memory.kept("/w", refused + COOL_DOWN), None);
}
