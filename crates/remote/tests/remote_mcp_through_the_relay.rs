//! devpit's MCP from another machine through devpit-app's own relay, on
//! loopback: the machine's relay client as the desktop runs it, the client's
//! `Relay::view` and `mcp::forward` as `devpit mcp --remote` runs them, and
//! every frame the relay carried captured to a file.
//!
//! It needs a devpit-app checkout and builds its relay, so it runs only when
//! asked:
//!
//! ```sh
//! DEVPIT_APP_DIR=../devpit-app cargo test -p devpit-remote --test remote_mcp_through_the_relay
//! ```

mod support;

use std::io::{BufRead as _, BufReader};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use devpit_remote::mcp::{MOST_FROM_CLIENT, MOST_FROM_MACHINE};
use devpit_remote::noise::{hex, Keys};
use devpit_remote::relay::timeout;
use devpit_remote::{Link, LinkError, Relay};
use serde_json::Value;
use support::{cut_at_once, none_readable, text, Machine, HOST, PROJECT, QUESTION};
use tungstenite::stream::MaybeTlsStream;

type RelayLink = Link<MaybeTlsStream<TcpStream>>;

/// The relay, stopped with the test.
struct Running(Child);

impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
    }
}

/// devpit-app's loopback relay: its address and its sessions' tokens.
fn relay(app: &str, capture: &Path) -> (Running, Value) {
    let mut child = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "devpit-app-relay",
            "--example",
            "loopback",
            "--",
        ])
        .arg(capture)
        .current_dir(app)
        .stdout(Stdio::piped())
        .spawn()
        .expect("cargo run the relay");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout"))
        .read_line(&mut line)
        .expect("the relay's line");
    (Running(child), serde_json::from_str(&line).expect("json"))
}

/// The machine online on the relay, each device joined on a thread.
fn online(machine: Arc<Machine>, relay: Relay) {
    std::thread::spawn(move || loop {
        let Ok(mut control) = relay.online() else {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        };
        while let Ok(next) = control.waiting() {
            let Some(splice) = next else { continue };
            let (machine, relay) = (machine.clone(), relay.clone());
            std::thread::spawn(move || {
                let Ok(socket) = relay.splice(&splice) else {
                    return;
                };
                timeout(&socket, Duration::from_millis(40));
                let within = Duration::from_secs(10);
                let answered = Link::answer(socket, &machine.keys, HOST, MOST_FROM_CLIENT, within);
                if let Ok(link) = answered {
                    machine.answer(link);
                }
            });
        }
    });
}

/// A device's session with the machine, as `devpit mcp --remote` makes it.
fn view(
    origin: &str,
    token: &str,
    keys: &Keys,
    machine: &[u8; 32],
) -> Result<RelayLink, LinkError> {
    let socket = Relay::new(origin, token, HOST)
        .view()
        .map_err(|_| LinkError::Gone)?;
    let within = Duration::from_secs(10);
    let link = Link::call(socket, keys, HOST, machine, MOST_FROM_MACHINE, within)?;
    timeout(link.socket(), Duration::from_millis(40));
    Ok(link)
}

/// Every frame the relay wrote down, either way.
fn captured(capture: &Path) -> Vec<Vec<u8>> {
    std::fs::read_to_string(capture)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| line.split_once(' '))
        .map(|(_, bytes)| {
            (0..bytes.len() / 2)
                .filter_map(|at| u8::from_str_radix(&bytes[at * 2..at * 2 + 2], 16).ok())
                .collect()
        })
        .collect()
}

fn refused_with(said: &Value, why: &str) -> bool {
    said["error"]["message"]
        .as_str()
        .unwrap_or_default()
        .contains(why)
}

#[test]
fn an_mcp_client_reads_whats_waiting_through_a_local_devpit_app_relay() {
    let Ok(app) = std::env::var("DEVPIT_APP_DIR") else {
        eprintln!("skipped: DEVPIT_APP_DIR names no devpit-app checkout");
        return;
    };
    let dir = std::env::temp_dir().join(format!("devpit-mcp-relay-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("dir");
    let capture = dir.join("frames.txt");
    let (_relay, tokens) = relay(&app, &capture);
    let token = |name: &str| tokens[name].as_str().expect("a token").to_owned();
    let origin = tokens["relay"].as_str().expect("relay").to_owned();
    let device = token("device");
    let machine = Machine::new();
    online(
        machine.clone(),
        Relay::new(&origin, &token("machine"), HOST),
    );
    let public = machine.keys.public;

    // Until the machine is online, the relay answers 4404.
    let began = Instant::now();
    while view(&origin, &device, &Keys::generate().unwrap(), &public)
        .is_err_and(|err| matches!(err, LinkError::Closed(4404)))
    {
        assert!(began.elapsed() < Duration::from_secs(30), "never online");
        std::thread::sleep(Duration::from_millis(200));
    }
    let reach = |keys: Keys| {
        let (origin, device) = (origin.clone(), device.clone());
        move || view(&origin, &device, &keys, &public).map_err(|err| err.to_string())
    };
    let paired = support::paired(machine.clone(), reach);

    // What waits on the person, read from the other machine.
    let waiting = paired.client.call(1, "devpit_waiting");
    assert!(text(&waiting).contains(QUESTION) && text(&waiting).contains(PROJECT));
    // A write, refused there before the app hears of it.
    let comment = paired.client.call(2, "devpit_comment");
    assert_eq!(comment["result"]["isError"], true);
    assert!(text(&comment).contains("read-only"));
    assert_eq!(*machine.asked.lock().unwrap(), ["waiting"]);

    // What the relay carried: ciphertext, which keys it could make open none of.
    let words = [
        PROJECT,
        QUESTION,
        "devpit_waiting",
        &paired.secret,
        "jsonrpc",
        "loopback",
    ];
    none_readable(&captured(&capture), &words);

    // Forgotten on the machine: cut at once, and refused after.
    cut_at_once(&paired.ends, || machine.forget());
    let said = paired.client.call(3, "devpit_waiting");
    assert!(refused_with(&said, "not paired"), "{said}");

    // Paired again, then remote MCP turned off: cut at once, refused after.
    *machine.secret.lock().unwrap() = Some(hex(&Keys::generate().unwrap().secret));
    let again = support::paired(machine.clone(), reach);
    assert!(text(&again.client.call(1, "devpit_projects")).contains(PROJECT));
    cut_at_once(&again.ends, || machine.on.store(false, Ordering::SeqCst));
    let said = again.client.call(2, "devpit_waiting");
    assert!(refused_with(&said, "remote MCP is off"), "{said}");

    // Another account's device is joined to no machine of this one.
    let stranger = view(
        &origin,
        &token("stranger"),
        &Keys::generate().unwrap(),
        &public,
    );
    assert!(matches!(stranger, Err(LinkError::Closed(4404))));
    let _ = std::fs::remove_dir_all(&dir);
}
