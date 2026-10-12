//! A machine that serves devpit's MCP to a client from outside the way the
//! desktop does — who the device is by its key, pairing by the one-time
//! secret, the switch, the allowlisted MCP — over whatever carries its link.
//! And the client's half: pairing once, then `mcp::forward`.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use devpit_remote::mcp::{self, MOST_FROM_CLIENT};
use devpit_remote::noise::{hex, Handshake, Keys};
use devpit_remote::{Heard, Link};
use serde_json::{json, Value};

pub const HOST: &str = "host_01MCPLOOPBACK";
/// Words only the two ends may ever see.
pub const PROJECT: &str = "loopback secret project";
pub const QUESTION: &str = "cargo publish --token sekrit";

pub struct Machine {
    pub keys: Keys,
    pub secret: Mutex<Option<String>>,
    pub paired: Mutex<Vec<[u8; 32]>>,
    /// Settings → Remote → Allow remote MCP.
    pub on: AtomicBool,
    /// What the app was asked, by method.
    pub asked: Mutex<Vec<String>>,
    /// Told when a client's session ends on the machine.
    pub ended: Mutex<Option<Sender<Instant>>>,
}

impl Machine {
    pub fn new() -> Arc<Machine> {
        Arc::new(Machine {
            keys: Keys::generate().expect("keys"),
            secret: Mutex::new(Some(hex(&Keys::generate().expect("random").secret))),
            paired: Mutex::new(Vec::new()),
            on: AtomicBool::new(true),
            asked: Mutex::new(Vec::new()),
            ended: Mutex::new(None),
        })
    }

    pub fn secret(&self) -> String {
        self.secret
            .lock()
            .unwrap()
            .clone()
            .expect("a secret on offer")
    }

    /// Forgets every client, as Settings → Remote → Forget does.
    pub fn forget(&self) {
        self.paired.lock().unwrap().clear();
    }

    pub fn ends(&self) -> Receiver<Instant> {
        let (tell, heard) = channel();
        *self.ended.lock().unwrap() = Some(tell);
        heard
    }

    /// The app behind the allowlist: what waits on the person, and a board.
    fn ask(&self, method: &str, _: Value) -> Result<Value, String> {
        self.asked.lock().unwrap().push(method.to_owned());
        match method {
            "waiting" => Ok(json!({ "questions": [
                { "from": "terminal", "project": PROJECT, "tool": "Bash", "input": QUESTION }
            ] })),
            "projects" => Ok(json!({ "projects": [{ "id": "p1", "name": PROJECT }] })),
            _ => Err(format!("`{method}` is not read from outside the machine")),
        }
    }

    fn known(&self, peer: &[u8; 32]) -> bool {
        self.paired.lock().unwrap().contains(peer)
    }

    /// One client's link, from its handshake to its end.
    pub fn answer<S: Read + Write>(&self, mut link: Link<S>) {
        let peer = link.peer();
        if !self.known(&peer) {
            let said = first(&mut link);
            let mut offered = self.secret.lock().unwrap();
            let pairs = said["t"] == "pair"
                && offered
                    .as_deref()
                    .is_some_and(|secret| said["secret"] == secret);
            if pairs {
                offered.take();
            }
            drop(offered);
            if !pairs {
                refuse(
                    &mut link,
                    "this device is not paired with this machine — pair it again",
                );
                return;
            }
            self.paired.lock().unwrap().push(peer);
        }
        if !self.on.load(Ordering::SeqCst) {
            refuse(
                &mut link,
                "remote MCP is off on that machine — allow it in Settings → Remote",
            );
            return;
        }
        let mut still = || self.on.load(Ordering::SeqCst) && self.known(&peer);
        let mut answer = |line: &str| {
            devpit_agentapi::remote_mcp::handle(line, &|method, params| self.ask(method, params))
        };
        mcp::serve(&mut link, &mut still, &mut answer, Duration::from_secs(90));
        link.close();
        if let Some(tell) = self.ended.lock().unwrap().as_ref() {
            let _ = tell.send(Instant::now());
        }
    }
}

fn first<S: Read + Write>(link: &mut Link<S>) -> Value {
    let began = Instant::now();
    while began.elapsed() < Duration::from_secs(10) {
        match link.receive() {
            Ok(Heard::Message(bytes)) => return serde_json::from_slice(&bytes).unwrap_or_default(),
            Ok(_) => {}
            Err(_) => break,
        }
    }
    Value::Null
}

fn refuse<S: Read + Write>(link: &mut Link<S>, why: &str) {
    let _ = link.send(json!({ "t": "refused", "why": why }).to_string().as_bytes());
    std::thread::sleep(Duration::from_millis(50));
}

/// Pairs a client's link with `secret`, as `devpit mcp --remote pair` does.
pub fn pair<S: Read + Write>(mut link: Link<S>, secret: &str) -> Result<(), String> {
    let pairing = json!({ "t": "pair", "secret": secret, "name": "devpit MCP on loopback" });
    let ping = json!({ "jsonrpc": "2.0", "id": "pair", "method": "ping" });
    link.send(pairing.to_string().as_bytes())
        .map_err(|err| err.to_string())?;
    link.send(ping.to_string().as_bytes())
        .map_err(|err| err.to_string())?;
    let said = first(&mut link);
    link.close();
    match said["id"] == "pair" && said.get("result").is_some() {
        true => Ok(()),
        false => Err(said.to_string()),
    }
}

pub struct Paired {
    pub machine: Arc<Machine>,
    pub client: Client,
    pub secret: String,
    /// When each of the client's sessions ended on the machine.
    pub ends: Receiver<Instant>,
}

/// A client paired with `machine` and running, its sessions made by the
/// connector `reach` returns for the client's keys.
pub fn paired<S, C>(machine: Arc<Machine>, reach: impl Fn(Keys) -> C) -> Paired
where
    S: Read + Write + 'static,
    C: FnMut() -> Result<Link<S>, String> + Send + 'static,
{
    let keys = Keys::generate().expect("keys");
    let secret = machine.secret();
    let ends = machine.ends();
    let mut connect = reach(keys.clone());
    pair(connect().expect("reached"), &secret).expect("paired");
    // The pairing's own session, over before the client's begins.
    ends.recv_timeout(Duration::from_secs(10))
        .expect("pairing ended");
    Paired {
        client: Client::start(reach(keys)),
        machine,
        secret,
        ends,
    }
}

/// Cut by `cut`: the session ends on the machine within a second of it.
pub fn cut_at_once(ends: &Receiver<Instant>, cut: impl FnOnce()) {
    let at = Instant::now();
    cut();
    let ended = ends.recv_timeout(Duration::from_secs(2)).expect("cut");
    assert!(ended - at < Duration::from_secs(1), "cut at once");
    // The client's side is closed next; it asks again a moment later.
    std::thread::sleep(Duration::from_millis(300));
}

/// A client running `mcp::forward` on a thread: lines in, answers out.
pub struct Client {
    pub lines: Sender<String>,
    pub out: Receiver<String>,
}

impl Client {
    pub fn start<S, C>(mut connect: C) -> Client
    where
        S: Read + Write + 'static,
        C: FnMut() -> Result<Link<S>, String> + Send + 'static,
    {
        let (lines, heard) = channel::<String>();
        let (tell, out) = channel();
        std::thread::spawn(move || {
            let mut out = |line: &str| tell.send(line.to_owned()).is_ok();
            mcp::forward(&heard, &mut connect, &mut out);
        });
        Client { lines, out }
    }

    /// Request `id`'s answer, from a `method` call with `params`.
    pub fn ask(&self, id: u64, method: &str, params: Value) -> Value {
        let line = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        self.lines.send(line.to_string()).expect("client running");
        let began = Instant::now();
        while began.elapsed() < Duration::from_secs(20) {
            if let Ok(line) = self.out.recv_timeout(Duration::from_millis(100)) {
                let said: Value = serde_json::from_str(&line).expect("json");
                if said["id"] == id {
                    return said;
                }
            }
        }
        panic!("no answer to {id} ({method})");
    }

    pub fn call(&self, id: u64, tool: &str) -> Value {
        self.ask(id, "tools/call", json!({ "name": tool, "arguments": {} }))
    }
}

/// The text a tool call answered with.
pub fn text(said: &Value) -> &str {
    said["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
}

/// Every frame `carried`, checked for the words that crossed and against a
/// session of keys made for the purpose: it opens none.
pub fn none_readable(carried: &[Vec<u8>], words: &[&str]) {
    assert!(carried.len() > 6, "only {} frames", carried.len());
    for frame in carried {
        let seen = String::from_utf8_lossy(frame);
        for word in words {
            assert!(!seen.contains(word), "`{word}` crossed in the clear");
        }
    }
    let (one, other) = (Keys::generate().unwrap(), Keys::generate().unwrap());
    let mut device = Handshake::initiator(&one, HOST).unwrap();
    let mut machine = Handshake::responder(&other, HOST).unwrap();
    machine.read(&device.write().unwrap()).unwrap();
    device.read(&machine.write().unwrap()).unwrap();
    machine.read(&device.write().unwrap()).unwrap();
    let mut sessions = [
        device.into_session(MOST_FROM_CLIENT).unwrap(),
        machine.into_session(MOST_FROM_CLIENT).unwrap(),
    ];
    for frame in carried {
        for session in &mut sessions {
            assert!(
                session.open(frame).is_err(),
                "a frame opened with other keys"
            );
        }
    }
}
