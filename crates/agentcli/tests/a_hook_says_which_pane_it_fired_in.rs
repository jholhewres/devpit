//! The hook command, run as a shell actually runs it.
//!
//! This is a shell script inside a JSON string inside a settings file, and
//! every layer of that has its own quoting. A unit test on the generated text
//! only proves the text; the thing worth proving is that a shell handed it
//! posts where it should, with the payload intact.
//!
//! What the pane buys: the process table can say *which* agent is open in a
//! terminal, but not whether it is working or has stopped and is waiting for
//! somebody. Only the agent knows that, and its hook fires three processes
//! below the shell — so the pane travels in the environment and comes back in
//! the query string.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};

const PAYLOAD: &str = r#"{"hook_event_name":"Stop","session_id":"s1","cwd":"/tmp"}"#;

/// Stands in for the secret the app makes at startup.
const SECRET: &str = "a-secret-only-this-run-knows";

#[test]
fn the_pane_reaches_the_listener_and_the_payload_survives() {
    let Some((target, body, carried)) = posted(Some("leaf_01TEST")) else {
        eprintln!("skipped: no shell or no curl");
        return;
    };
    assert_eq!(target, "/hook?pane=leaf_01TEST");
    assert_eq!(
        body, PAYLOAD,
        "the agent's own report was altered on the way"
    );
    // Read from the file by curl, never typed into the command: this is what
    // the listener checks before it hears anything.
    assert_eq!(
        carried.as_deref(),
        Some(SECRET),
        "the hook posted without the secret"
    );
}

/// A headless turn belongs to a card and runs in no pane at all. The same
/// command has to post exactly the URL it always did — `${VAR:+…}` expands to
/// nothing, and nothing is what a turn with no terminal should add.
#[test]
fn a_turn_with_no_pane_posts_what_it_always_did() {
    let Some((target, _, _)) = posted(None) else {
        eprintln!("skipped: no shell or no curl");
        return;
    };
    assert_eq!(target, "/hook");
}

/// Runs the real hook command against a real socket, and reports what arrived.
fn posted(pane: Option<&str>) -> Option<(String, String, Option<String>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).ok()?;
    let port = listener.local_addr().ok()?.port();

    let home = tempfile::tempdir().ok()?;
    let endpoint = home.path().join("hook-endpoint");
    std::fs::write(&endpoint, format!("http://127.0.0.1:{port}/hook")).ok()?;

    let auth = home.path().join("hook-auth");
    std::fs::write(
        &auth,
        format!("{}: {SECRET}\n", devpit_agentcli::HOOK_HEADER),
    )
    .ok()?;

    let settings = devpit_agentcli::settings_json(&endpoint, &auth);
    let command = one_command(&settings, "Stop")?;

    let heard = std::thread::spawn(move || {
        let (stream, _) = listener.accept().ok()?;
        read_one(stream)
    });

    let mut shell = Command::new("sh")
        .args(["-c", &command])
        .env_remove("DEVPIT_PANE")
        .envs(pane.map(|name| ("DEVPIT_PANE", name)))
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    shell.stdin.take()?.write_all(PAYLOAD.as_bytes()).ok()?;
    let _ = shell.wait();

    heard.join().ok()?
}

/// Reads the request line and the body off one connection, then answers.
fn read_one(stream: std::net::TcpStream) -> Option<(String, String, Option<String>)> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut first = String::new();
    reader.read_line(&mut first).ok()?;
    let target = first.split_whitespace().nth(1)?.to_owned();

    let mut length = 0usize;
    let mut carried = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            break;
        }
        let line = line.trim_end().to_ascii_lowercase();
        if line.is_empty() {
            break;
        }
        if let Some(value) = line.strip_prefix("content-length:") {
            length = value.trim().parse().ok()?;
        }
        if let Some(value) = line.strip_prefix(&format!("{}:", devpit_agentcli::HOOK_HEADER)) {
            carried = Some(value.trim().to_owned());
        }
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).ok()?;

    // Answered, because the hook waits for it and a hook left hanging is a
    // turn left hanging.
    let mut back = stream;
    let _ = back.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n");
    let _ = back.flush();

    Some((target, String::from_utf8(body).ok()?, carried))
}

/// Digs one event's shell command out of the generated settings.
///
/// By hand rather than with a JSON crate the crate does not otherwise need
/// here — the shape is ours and one line deep.
fn one_command(settings: &str, event: &str) -> Option<String> {
    let after = settings.split_once(&format!("\"{event}\":"))?.1;
    let opened = after.find("\"command\":\"")? + "\"command\":\"".len();
    let rest = &after[opened..];

    let mut command = String::new();
    let mut escaped = false;
    for character in rest.chars() {
        if escaped {
            command.push(match character {
                'n' => '\n',
                't' => '\t',
                other => other,
            });
            escaped = false;
            continue;
        }
        match character {
            '\\' => escaped = true,
            '"' => return Some(command),
            other => command.push(other),
        }
    }
    None
}
