use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tungstenite::{Message, WebSocket};

use super::*;
use crate::local_server::{Door, Place};
use crate::remote_http::Request;

#[test]
fn a_frame_is_the_handle_the_kind_and_the_bytes() {
    let wire = encode(258, kind::INPUT, b"ab");
    assert_eq!(wire, [2, 1, 0, 0, kind::INPUT, b'a', b'b']);
    assert_eq!(decode(&wire), Some((258, kind::INPUT, &b"ab"[..])));
    assert_eq!(decode(&[1, 0, 0]), None);
}

fn place() -> Place {
    Place {
        port: 4100,
        name: "0123456789abcdef0123456789abcdef.devpit.localhost".to_owned(),
    }
}

fn get(path: &str, host: &str, origin: Option<&str>) -> Request {
    let mut headers = std::collections::HashMap::new();
    headers.insert("host".to_owned(), host.to_owned());
    if let Some(origin) = origin {
        headers.insert("origin".to_owned(), origin.to_owned());
    }
    Request {
        method: "GET".to_owned(),
        path: path.to_owned(),
        headers,
        body: Vec::new(),
    }
}

const TICKET: &str = "ab12";

#[test]
fn the_desktop_window_opens_the_panes_socket_with_its_ticket_whatever_its_origin() {
    for origin in [
        Some("tauri://localhost"),
        Some("http://tauri.localhost"),
        None,
    ] {
        let request = get("/panes", "127.0.0.1:4100", origin);
        assert_eq!(
            ticket_door(&request, "k=ab12", &place(), TICKET),
            Some(Door::In),
            "{origin:?}"
        );
    }
    let request = get("/panes", "127.0.0.1:4100", Some("tauri://localhost"));
    assert_eq!(
        ticket_door(&request, "k=wrong", &place(), TICKET),
        Some(Door::Unauthorized)
    );
    assert_eq!(
        ticket_door(&request, "k=", &place(), TICKET),
        Some(Door::Unauthorized)
    );
}

#[test]
fn the_ticket_is_no_way_past_the_host_check_or_into_anything_else() {
    let rebound = get("/panes", "evil.example:4100", None);
    assert_eq!(
        ticket_door(&rebound, "k=ab12", &place(), TICKET),
        Some(Door::Foreign)
    );
    let rpc = get("/rpc/project_list", "127.0.0.1:4100", None);
    assert_eq!(ticket_door(&rpc, "k=ab12", &place(), TICKET), None);
    let no_ticket = get("/panes", "127.0.0.1:4100", None);
    assert_eq!(ticket_door(&no_ticket, "", &place(), TICKET), None);
}

#[tokio::test]
async fn a_flood_waits_for_its_acks_and_a_release_lets_it_go() {
    let flow = Arc::new(crate::live::Flow::default());
    flow.sent(crate::live::Flow::MOST + 1);
    let waiting = {
        let flow = Arc::clone(&flow);
        tokio::spawn(async move { flow.room().await })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(!waiting.is_finished(), "past the window, it waits");
    flow.ack(2);
    tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .expect("an ack lets it on")
        .expect("joined");

    flow.sent(crate::live::Flow::MOST * 2);
    let waiting = {
        let flow = Arc::clone(&flow);
        tokio::spawn(async move { flow.room().await })
    };
    flow.release();
    tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .expect("a release lets it on")
        .expect("joined");
}

/// Panes that answer by hand: what the socket asked, and a sink per attach.
#[derive(Default)]
struct Fake {
    said: Mutex<Vec<String>>,
    sinks: Mutex<Vec<(String, Arc<dyn crate::live::Sink>)>>,
}

impl Panes for Fake {
    fn attach(
        &self,
        _label: &str,
        client_id: String,
        asked: Asked,
        sink: Arc<dyn crate::live::Sink>,
        _end: Ended,
    ) {
        self.said.lock().unwrap().push(format!(
            "attach {} {} {}x{}",
            asked.pane_id,
            client_id.contains(':'),
            asked.rows,
            asked.cols
        ));
        self.sinks.lock().unwrap().push((asked.pane_id, sink));
    }
    fn detach(&self, pane_id: &str, _client_id: &str) {
        self.said.lock().unwrap().push(format!("detach {pane_id}"));
    }
    fn input(&self, pane_id: &str, bytes: &[u8]) {
        self.said.lock().unwrap().push(format!(
            "input {pane_id} {}",
            String::from_utf8_lossy(bytes)
        ));
    }
    fn resize(&self, _pane_id: &str, rows: u16, cols: u16) -> Option<(u16, u16)> {
        Some((rows, cols.min(90)))
    }
    fn ack(&self, pane_id: &str, bytes: u32) {
        self.said
            .lock()
            .unwrap()
            .push(format!("ack {pane_id} {bytes}"));
    }
    fn older(&self, _pane_id: &str) -> bool {
        false
    }
}

fn connected(panes: Arc<Fake>) -> WebSocket<TcpStream> {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bound");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accepted");
        let mut reader = std::io::BufReader::new(stream.try_clone().expect("clone"));
        let request = crate::remote_http::read(&mut reader).expect("request");
        serve(stream, &request, panes);
    });
    let stream = TcpStream::connect(("127.0.0.1", port)).expect("connected");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    let (ws, _) = tungstenite::client::client(format!("ws://127.0.0.1:{port}/panes"), stream)
        .expect("upgraded");
    ws
}

fn binary(ws: &mut WebSocket<TcpStream>) -> Vec<u8> {
    loop {
        match ws.read().expect("a frame") {
            Message::Binary(bytes) => return bytes.to_vec(),
            Message::Ping(_) | Message::Pong(_) => {}
            other => panic!("not binary: {other:?}"),
        }
    }
}

fn wait_for(fake: &Fake, line: &str) {
    for _ in 0..200 {
        if fake.said.lock().unwrap().iter().any(|one| one == line) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("never said {line:?}: {:?}", fake.said.lock().unwrap());
}

#[test]
fn a_page_attaches_types_reads_and_resizes_a_pane_by_its_handle() {
    let fake = Arc::new(Fake::default());
    let mut ws = connected(Arc::clone(&fake));
    let attach = br#"{"projectId":"prj_1","paneId":"leaf_1","rows":24,"cols":80}"#;
    ws.send(Message::binary(encode(7, kind::ATTACH, attach)))
        .expect("sent");
    wait_for(&fake, "attach leaf_1 true 24x80");

    ws.send(Message::binary(encode(7, kind::INPUT, b"ls\r")))
        .expect("sent");
    wait_for(&fake, "input leaf_1 ls\r");
    ws.send(Message::binary(encode(
        7,
        kind::ACK,
        &65536u32.to_le_bytes(),
    )))
    .expect("sent");
    wait_for(&fake, "ack leaf_1 65536");

    let sink = fake.sinks.lock().unwrap()[0].1.clone();
    assert!(sink.acks());
    assert!(sink.send(crate::live::Out::Replay, b"old".to_vec()));
    assert!(sink.send(crate::live::Out::Output, b"new".to_vec()));
    assert_eq!(binary(&mut ws), encode(7, kind::REPLAY, b"old"));
    assert_eq!(binary(&mut ws), encode(7, kind::OUTPUT, b"new"));

    let mut size = 30u16.to_le_bytes().to_vec();
    size.extend_from_slice(&100u16.to_le_bytes());
    ws.send(Message::binary(encode(7, kind::RESIZE, &size)))
        .expect("sent");
    let mut applied = 30u16.to_le_bytes().to_vec();
    applied.extend_from_slice(&90u16.to_le_bytes());
    assert_eq!(binary(&mut ws), encode(7, kind::RESIZE, &applied));

    ws.send(Message::binary(encode(7, kind::OLDER, b"")))
        .expect("sent");
    assert_eq!(binary(&mut ws), encode(7, kind::OLDER, &0u64.to_le_bytes()));
}

#[test]
fn a_socket_that_closes_lets_every_pane_it_held_go() {
    let fake = Arc::new(Fake::default());
    let mut ws = connected(Arc::clone(&fake));
    for (handle, pane) in [(1u32, "leaf_a"), (2, "leaf_b")] {
        let attach = format!(r#"{{"projectId":"p","paneId":"{pane}","rows":1,"cols":1}}"#);
        ws.send(Message::binary(encode(
            handle,
            kind::ATTACH,
            attach.as_bytes(),
        )))
        .expect("sent");
    }
    ws.send(Message::binary(encode(2, kind::DETACH, b"")))
        .expect("sent");
    wait_for(&fake, "detach leaf_b");
    ws.close(None).expect("closed");
    wait_for(&fake, "detach leaf_a");
    let detached = fake
        .said
        .lock()
        .unwrap()
        .iter()
        .filter(|one| one.starts_with("detach"))
        .count();
    assert_eq!(detached, 2, "each pane let go once");
    let sink = fake.sinks.lock().unwrap()[0].1.clone();
    for _ in 0..200 {
        if !sink.send(crate::live::Out::Output, b"late".to_vec()) {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("a sink outlived its socket");
}
