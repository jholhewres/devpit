use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use devpit_agentcli::{AgentSession, Kind, Status};
use devpit_rpc::{Doing, LayoutNode};

use super::*;
use crate::card_sessions::card_sessions;
use crate::post::Posted;
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

/// What a background session's hook posts when it stops on a person: the
/// `Notification` shape the CLI sends, with no pane in the query.
const BACKGROUND_WAITING: &str = r#"{"hook_event_name":"Notification","session_id":"s-bg","cwd":"/w/card","message":"Claude needs your permission to use Bash","notification_type":"permission_prompt"}"#;

#[test]
fn a_background_session_waiting_reaches_its_card() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store_path = dir.path().join("state.db");
    let store = Store::open(&store_path).expect("store");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    store
        .link_session(&card, "a1b2", "s-bg", None, Some("/w/card"))
        .expect("link");
    let sink = Recorded {
        store_path,
        activities: Mutex::default(),
        said: Mutex::default(),
    };

    let posted = Posted {
        body: BACKGROUND_WAITING.to_owned(),
        pane: None,
    };
    hear_post(&sink, &posted, next_seq());

    let heard = sink.activities.lock().expect("activities").happening(&card);
    assert_eq!(heard.activity, Some(Doing::Waiting));
    assert_eq!(heard.sessions[0].kind, SessionKind::Background);
    assert_eq!(heard.sessions[0].reference, "s-bg");
    assert!(sink
        .said
        .lock()
        .expect("said")
        .iter()
        .any(|(channel, _)| channel == "card:happening"));

    // Combined when read: while the CLI lists it, its hook's word stands; once
    // the CLI no longer lists it, it is gone whatever it said last.
    let listed = AgentSession {
        session_id: "s-bg".to_owned(),
        short_id: Some("a1b2".to_owned()),
        name: None,
        cwd: "/w/card".to_owned(),
        pid: None,
        started_at: None,
        kind: Kind::Background,
        status: Status::Busy,
    };
    let read = card_sessions(&store, &project, &card, &[listed], &heard);
    assert_eq!(read[0].state, Some(Doing::Waiting));
    let unlisted = card_sessions(&store, &project, &card, &[], &heard);
    assert_eq!(unlisted[0].state, Some(Doing::Gone));
}

#[test]
fn a_hook_from_an_archived_cards_pane_does_not_bring_it_back() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store_path = dir.path().join("state.db");
    let store = Store::open(&store_path).expect("store");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &column, "a card", "")
        .expect("card");
    let tree = serde_json::to_string(&LayoutNode::leaf(LEAF, "s:leaf")).expect("tree");
    store
        .set_pane_layout(&project, &tab_for_card(&card), &tree, LEAF)
        .expect("layout");
    let sink = Recorded {
        store_path,
        activities: Mutex::default(),
        said: Mutex::default(),
    };
    let posted = Posted {
        body: PAYLOAD.to_owned(),
        pane: Some(LEAF.to_owned()),
    };
    hear_post(&sink, &posted, next_seq());
    assert!(!sink
        .activities
        .lock()
        .expect("lock")
        .happening(&card)
        .sessions
        .is_empty());

    // Archived: the card forgets its sessions, and the agent keeps talking.
    store.archive_card(&card).expect("archive");
    crate::card_activity::forget_card(&mut sink.activities.lock().expect("lock"), &card);
    sink.said.lock().expect("said").clear();
    hear_post(&sink, &posted, next_seq());

    assert!(sink
        .activities
        .lock()
        .expect("lock")
        .happening(&card)
        .sessions
        .is_empty());
    assert!(!sink
        .said
        .lock()
        .expect("said")
        .iter()
        .any(|(channel, _)| channel == "card:happening"));
}
