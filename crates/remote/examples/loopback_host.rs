//! The machine's side of the Remote through the relay, without the window:
//! the relay client and the Noise session the desktop runs, answering a few
//! requests the way the desktop would. For the end-to-end test in
//! `web/src/remote/relay.loopback.test.ts`, against devpit-app's relay on
//! loopback.
//!
//! ```sh
//! cargo run -p devpit-remote --example loopback_host -- <relay origin> <session token>
//! ```
//!
//! It prints one line of JSON — its id, its key and a pairing secret, what
//! the pairing QR code carries — then serves. `forget` on stdin unpairs every
//! device, as Settings → Remote → Forget does: their connections end at once.

use std::io::{BufRead as _, Write as _};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use devpit_remote::noise::{hex, Keys};
use devpit_remote::{Heard, Link, Relay};
use serde_json::{json, Value};

const HOST: &str = "host_01LOOPBACK";
/// A chat long enough to cross in several frames.
const LONG: usize = 150_000;

#[derive(Default)]
struct Paired {
    devices: Vec<([u8; 32], String)>,
    secret: Option<String>,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (origin, token) = (
        args.next().expect("relay origin"),
        args.next().expect("token"),
    );
    let keys = Keys::generate().expect("keys");
    let secret = hex(&Keys::generate().expect("random").secret);
    let paired = Arc::new(Mutex::new(Paired {
        secret: Some(secret.clone()),
        ..Paired::default()
    }));
    println!(
        "{}",
        json!({ "host": HOST, "key": hex(&keys.public), "secret": secret })
    );
    let _ = std::io::stdout().flush();

    let forgetting = paired.clone();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines().map_while(Result::ok) {
            if line.trim() == "forget" {
                forgetting.lock().unwrap().devices.clear();
            }
        }
    });

    let relay = Relay::new(&origin, &token, HOST);
    loop {
        let Ok(mut control) = relay.online() else {
            std::thread::sleep(Duration::from_millis(200));
            continue;
        };
        eprintln!("online");
        while let Ok(next) = control.waiting() {
            if let Some(splice) = next {
                let (relay, keys, paired) = (relay.clone(), keys.clone(), paired.clone());
                std::thread::spawn(move || serve(&relay, &keys, &paired, &splice));
            }
        }
    }
}

fn send(link: &mut Link<impl std::io::Read + std::io::Write>, said: &Value) -> bool {
    link.send(said.to_string().as_bytes()).is_ok()
}

fn serve(relay: &Relay, keys: &Keys, paired: &Mutex<Paired>, splice: &str) {
    let Ok(socket) = relay.splice(splice) else {
        return;
    };
    devpit_remote::relay::timeout(&socket, Duration::from_millis(40));
    let Ok(mut link) = Link::answer(socket, keys, HOST, 256 * 1024, Duration::from_secs(10)) else {
        return;
    };
    let peer = link.peer();
    let known = paired
        .lock()
        .unwrap()
        .devices
        .iter()
        .find(|(key, _)| *key == peer)
        .map(|(_, name)| name.clone());
    let name = match known {
        Some(name) => name,
        None => match first(&mut link) {
            Some(said) if said["t"] == "pair" && taken(paired, &said["secret"]) => {
                let name = said["name"].as_str().unwrap_or("A device").to_owned();
                paired.lock().unwrap().devices.push((peer, name.clone()));
                name
            }
            _ => {
                let why = "this device is not paired with this machine — pair it again";
                send(&mut link, &json!({ "t": "refused", "why": why }));
                return link.close();
            }
        },
    };
    let welcome = json!({ "t": "welcome", "device": name, "host": "loopback", "typing": false, "answering": false });
    if !send(&mut link, &welcome) {
        return;
    }
    loop {
        // Forgotten: the session and its keys end here, and the relay closes
        // the device's side when this one goes.
        if !paired
            .lock()
            .unwrap()
            .devices
            .iter()
            .any(|(key, _)| *key == peer)
        {
            return link.close();
        }
        let said = match link.receive() {
            Ok(Heard::Message(bytes)) => {
                serde_json::from_slice::<Value>(&bytes).unwrap_or_default()
            }
            Ok(Heard::Alive | Heard::Nothing) => continue,
            Err(_) => return,
        };
        let answer = match said["t"].as_str() {
            Some("projects") => json!({ "t": "projects", "projects": [
                { "id": "p1", "name": "loopback secret project", "group": null, "terminals": [] }
            ] }),
            Some("drafts") => json!({ "t": "drafts", "drafts": [
                { "profile": "default", "name": "long-session", "text": "x".repeat(LONG) }
            ] }),
            Some("ping") => json!({ "t": "pong" }),
            _ => json!({ "t": "failed", "why": "not in this loopback" }),
        };
        if !send(&mut link, &answer) {
            return;
        }
    }
}

fn first(link: &mut Link<impl std::io::Read + std::io::Write>) -> Option<Value> {
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(10) {
        match link.receive() {
            Ok(Heard::Message(bytes)) => return serde_json::from_slice(&bytes).ok(),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
    None
}

fn taken(paired: &Mutex<Paired>, secret: &Value) -> bool {
    let mut paired = paired.lock().unwrap();
    let matches = paired.secret.is_some() && paired.secret.as_deref() == secret.as_str();
    if matches {
        paired.secret = None;
    }
    matches
}
