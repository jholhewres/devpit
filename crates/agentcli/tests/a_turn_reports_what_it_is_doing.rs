//! The hook round-trip, against the CLI and a socket of our own.
//!
//! Everything else about hooks is shape-checking against a recorded payload.
//! This is the one that proves the loop closes: settings written by us, read by
//! the agent, posted back to a port we are listening on, while the turn runs.
//!
//! Opt-in behind `DEVPIT_LIVE_TURN`: it runs a real turn, which costs money.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener};
use std::sync::mpsc;

#[test]
fn a_real_turn_posts_its_hooks_to_us() {
    if std::env::var_os("DEVPIT_LIVE_TURN").is_none() || !devpit_agentcli::available() {
        eprintln!("skipped: set DEVPIT_LIVE_TURN=1 to run a real turn");
        return;
    }

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
    let address = listener.local_addr().expect("address");

    let (tx, rx) = mpsc::channel::<String>();
    std::thread::spawn(move || {
        for stream in listener.incoming().flatten() {
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let line = line.trim_end();
                if line.is_empty() {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; length];
            if length > 0 && reader.read_exact(&mut body).is_ok() {
                let _ = tx.send(String::from_utf8_lossy(&body).into_owned());
            }
            let mut stream = stream;
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
        }
    });

    // The endpoint on disk, exactly as the app publishes it.
    let dir = tempfile::tempdir().expect("tempdir");
    let endpoint = devpit_agentcli::endpoint_file(dir.path());
    std::fs::write(&endpoint, format!("http://{address}/hook")).expect("publish");

    let settings = dir.path().join("hooks.json");
    std::fs::write(&settings, devpit_agentcli::settings_json(&endpoint)).expect("settings");

    let outcome = devpit_agentcli::run_turn(
        &devpit_agentcli::Turn {
            prompt: "Run the shell command `echo hello`, then reply with the word done.",
            cwd: dir.path(),
            agents: None,
            schema: None,
            budget_usd: Some(0.50),
            model: Some("claude-haiku-4-5-20251001"),
            settings: settings.to_str(),
        },
        |_| {},
    )
    .expect("the turn ran");

    // Whatever arrived by the time the turn ended.
    let mut heard = Vec::new();
    while let Ok(payload) = rx.try_recv() {
        if let Some(happening) = devpit_agentcli::read_hook(&payload) {
            heard.push(happening);
        }
    }

    assert!(
        !heard.is_empty(),
        "the turn ran ({}) but no hook reached us — the loop is open somewhere",
        outcome.result
    );
    assert!(
        heard.iter().all(|h| !h.session_id.is_empty()),
        "a hook arrived with no session to attribute it to: {heard:?}"
    );
}
