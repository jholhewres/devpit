use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use devpit_rpc::{Doing, LayoutNode};

use super::*;
use crate::sessions::tab_for_card;

/// What the listener's core said, and to where.
struct Recorded {
    store_path: PathBuf,
    activities: Mutex<Activities>,
    said: Mutex<Vec<(String, serde_json::Value)>>,
}

impl HookSink for Recorded {
    fn to_window<P: serde::Serialize + Clone>(&self, channel: &str, payload: P) {
        let value = serde_json::to_value(payload).expect("payload");
        self.said
            .lock()
            .expect("said")
            .push((channel.to_owned(), value));
    }

    fn ring(&self, _store: &Store, _ring: Ring<'_>, _pane: &str) {}

    fn store(&self) -> Option<Store> {
        Store::open(&self.store_path).ok()
    }

    fn activities(&self) -> &Mutex<Activities> {
        &self.activities
    }
}

const PAYLOAD: &str = r#"{"hook_event_name":"Stop","session_id":"s1","cwd":"/tmp"}"#;
const LEAF: &str = "leaf_01HOOK";

/// The real hook command for one event, out of the settings a turn is given.
fn command_for(settings: &str, event: &str) -> String {
    let settings: serde_json::Value = serde_json::from_str(settings).expect("settings");
    settings["hooks"][event][0]["hooks"][0]["command"]
        .as_str()
        .expect("command")
        .to_owned()
}

#[test]
fn a_hook_reaches_the_card() {
    for (tool, check) in [("sh", "true"), ("curl", "command -v curl")] {
        let found = Command::new("sh")
            .arg("-c")
            .arg(check)
            .status()
            .is_ok_and(|status| status.success());
        assert!(found, "a_hook_reaches_the_card needs {tool} on the PATH");
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let store_path = dir.path().join("state.db");
    let store = Store::open(&store_path).expect("store");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("root");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let tree = serde_json::to_string(&LayoutNode::leaf(LEAF, "s:leaf")).expect("tree");
    store
        .set_pane_layout(&project, &tab_for_card(&card), &tree, LEAF)
        .expect("layout");
    drop(store);

    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let endpoint = dir.path().join("hook-endpoint");
    let port = listener.local_addr().expect("address").port();
    std::fs::write(&endpoint, format!("http://127.0.0.1:{port}/hook")).expect("endpoint");

    let sink = Arc::new(Recorded {
        store_path,
        activities: Mutex::default(),
        said: Mutex::default(),
    });
    let core = Arc::clone(&sink);
    let served = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).expect("blocking");
                    let posted = read_request(&mut stream).expect("a post");
                    reply(&mut stream, "");
                    hear_post(core.as_ref(), &posted, next_seq());
                    return true;
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => return false,
            }
        }
    });

    let command = command_for(&devpit_agentcli::settings_json(&endpoint), "Stop");
    let mut hook = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .env("DEVPIT_PANE", LEAF)
        .stdin(Stdio::piped())
        .spawn()
        .expect("hook");
    hook.stdin
        .take()
        .expect("stdin")
        .write_all(PAYLOAD.as_bytes())
        .expect("payload");
    hook.wait().expect("hook ran");
    assert!(served.join().expect("served"), "the hook never posted");

    let said = sink.said.lock().expect("said");
    let (_, happening) = said
        .iter()
        .find(|(channel, _)| channel == "card:happening")
        .expect("the card heard it");
    assert_eq!(happening["cardId"], card.as_str());
    assert_eq!(happening["activity"], "done");
    assert_eq!(happening["sessions"][0]["ref"], LEAF);
    assert_eq!(
        happening["sessions"][0]["tabId"],
        tab_for_card(&card).as_str()
    );
    assert_eq!(
        sink.activities
            .lock()
            .expect("activities")
            .happening(&card)
            .activity,
        Some(Doing::Done)
    );
}
