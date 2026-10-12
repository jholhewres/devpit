//! devpit's MCP from another machine, end to end on loopback: the client's
//! `mcp::forward`, a splice that carries frames the way devpit.app's relay
//! does and keeps every one, and a machine answering with the allowlisted
//! MCP. `remote_mcp_through_the_relay.rs` runs the same against the relay
//! itself, when a devpit-app checkout is at hand.

mod support;

use std::net::{TcpListener, TcpStream};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use devpit_remote::mcp::{MOST_FROM_CLIENT, MOST_FROM_MACHINE};
use devpit_remote::noise::Keys;
use devpit_remote::Link;
use serde_json::json;
use support::{cut_at_once, none_readable, text, Machine, Paired, HOST, PROJECT, QUESTION};
use tungstenite::protocol::frame::coding::CloseCode;
use tungstenite::protocol::CloseFrame;
use tungstenite::{Message, WebSocket};

const READ_WAIT: Duration = Duration::from_millis(20);

/// The machine, taking each connection the splice opens for a client.
fn listening(machine: Arc<Machine>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        for stream in listener.incoming().map_while(Result::ok) {
            let machine = machine.clone();
            std::thread::spawn(move || {
                let Ok(ws) = tungstenite::accept(stream) else {
                    return;
                };
                let _ = ws.get_ref().set_read_timeout(Some(READ_WAIT));
                let answered = Link::answer(
                    ws,
                    &machine.keys,
                    HOST,
                    MOST_FROM_CLIENT,
                    Duration::from_secs(10),
                );
                if let Ok(link) = answered {
                    machine.answer(link);
                }
            });
        }
    });
    port
}

/// What stands where devpit.app's relay does: each client joined to a
/// connection of its own to the machine, a binary frame at a time, every
/// frame kept. When either end leaves, the other is closed with 4408.
struct Splice {
    port: u16,
    carried: Arc<Mutex<Vec<Vec<u8>>>>,
}

impl Splice {
    fn to(machine: u16) -> Splice {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().expect("addr").port();
        let carried = Arc::new(Mutex::new(Vec::new()));
        let kept = carried.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming().map_while(Result::ok) {
                let kept = kept.clone();
                std::thread::spawn(move || {
                    let Ok(device) = tungstenite::accept(stream) else {
                        return;
                    };
                    let joined = TcpStream::connect(("127.0.0.1", machine)).expect("machine");
                    let (host, _) = tungstenite::client("ws://127.0.0.1/", joined).expect("ws");
                    pump(device, host, &kept);
                });
            }
        });
        Splice { port, carried }
    }

    fn carried(&self) -> Vec<Vec<u8>> {
        self.carried.lock().unwrap().clone()
    }

    /// A client's session with the machine, through this splice.
    fn reach(
        &self,
        machine: [u8; 32],
    ) -> impl Fn(Keys) -> Box<dyn FnMut() -> Result<Link<TcpStream>, String> + Send> {
        let port = self.port;
        move |keys: Keys| {
            Box::new(move || {
                let stream =
                    TcpStream::connect(("127.0.0.1", port)).map_err(|err| err.to_string())?;
                let (ws, _) = tungstenite::client("ws://127.0.0.1/", stream)
                    .map_err(|err| err.to_string())?;
                let link = Link::call(
                    ws,
                    &keys,
                    HOST,
                    &machine,
                    MOST_FROM_MACHINE,
                    Duration::from_secs(10),
                )
                .map_err(|err| err.to_string())?;
                let _ = link.socket().get_ref().set_read_timeout(Some(READ_WAIT));
                Ok(link)
            })
        }
    }
}

/// A frame from one end to the other; false once that end has left.
fn carry(
    from: &mut WebSocket<TcpStream>,
    to: &mut WebSocket<TcpStream>,
    kept: &Mutex<Vec<Vec<u8>>>,
) -> bool {
    match from.read() {
        Ok(Message::Binary(frame)) => {
            kept.lock().unwrap().push(frame.to_vec());
            to.send(Message::Binary(frame)).is_ok()
        }
        Ok(Message::Close(_)) => false,
        Ok(_) => true,
        Err(tungstenite::Error::Io(err))
            if matches!(
                err.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) =>
        {
            true
        }
        Err(_) => false,
    }
}

fn pump(
    mut device: WebSocket<TcpStream>,
    mut host: WebSocket<TcpStream>,
    kept: &Mutex<Vec<Vec<u8>>>,
) {
    let _ = device.get_ref().set_read_timeout(Some(READ_WAIT));
    let _ = host.get_ref().set_read_timeout(Some(READ_WAIT));
    while carry(&mut device, &mut host, kept) && carry(&mut host, &mut device, kept) {}
    for end in [&mut device, &mut host] {
        let _ = end.close(Some(CloseFrame {
            code: CloseCode::from(4408),
            reason: "the other end left".into(),
        }));
        let _ = end.flush();
    }
}

/// A machine, a splice before it, and a client paired with it.
fn paired() -> (Paired, Splice) {
    let machine = Machine::new();
    let splice = Splice::to(listening(machine.clone()));
    let reach = splice.reach(machine.keys.public);
    (support::paired(machine, reach), splice)
}

#[test]
fn whats_waiting_crosses_to_the_other_machine_and_the_splice_reads_none_of_it() {
    let (paired, splice) = paired();
    let client = &paired.client;
    let hello = client.ask(1, "initialize", json!({ "protocolVersion": "2025-06-18" }));
    assert_eq!(hello["result"]["serverInfo"]["name"], "devpit");
    let listed = client.ask(2, "tools/list", json!({}));
    let names: Vec<&str> = listed["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert!(names.contains(&"devpit_waiting") && !names.contains(&"devpit_comment"));

    let waiting = client.call(3, "devpit_waiting");
    assert!(text(&waiting).contains(QUESTION) && text(&waiting).contains(PROJECT));

    none_readable(
        &splice.carried(),
        &[
            PROJECT,
            QUESTION,
            "devpit_waiting",
            &paired.secret,
            "jsonrpc",
            "loopback",
        ],
    );
}

#[test]
fn a_write_is_refused_from_the_other_machine_and_the_app_never_hears_of_it() {
    let (paired, _splice) = paired();
    for (id, tool) in [
        "devpit_comment",
        "devpit_move_card",
        "devpit_start_session",
        "devpit_send_draft",
    ]
    .into_iter()
    .enumerate()
    {
        let said = paired.client.call(id as u64 + 1, tool);
        assert_eq!(said["result"]["isError"], true, "{tool}");
        assert!(text(&said).contains("read-only"), "{tool}");
    }
    assert!(paired.machine.asked.lock().unwrap().is_empty());
}

#[test]
fn turning_remote_mcp_off_cuts_the_client_at_once_and_refuses_it_after() {
    let (paired, _splice) = paired();
    assert!(text(&paired.client.call(1, "devpit_waiting")).contains(QUESTION));
    cut_at_once(&paired.ends, || {
        paired.machine.on.store(false, Ordering::SeqCst)
    });
    let said = paired.client.call(2, "devpit_waiting");
    let why = said["error"]["message"].as_str().unwrap_or_default();
    assert!(why.contains("remote MCP is off"), "{said}");
}

#[test]
fn a_forgotten_client_is_cut_at_once_and_its_key_opens_nothing_after() {
    let (paired, _splice) = paired();
    assert!(text(&paired.client.call(1, "devpit_projects")).contains(PROJECT));
    cut_at_once(&paired.ends, || paired.machine.forget());
    let said = paired.client.call(2, "devpit_waiting");
    let why = said["error"]["message"].as_str().unwrap_or_default();
    assert!(why.contains("not paired"), "{said}");
}
