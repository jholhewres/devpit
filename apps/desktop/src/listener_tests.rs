use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
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

/// Runs the real hook command for one event, the way the CLI does, and
/// answers the post it made. `pane` is set only for an agent inside a pane.
fn posted_by_hook(dir: &Path, event: &str, payload: &str, pane: Option<&str>) -> Posted {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let endpoint = dir.join("hook-endpoint");
    let port = listener.local_addr().expect("address").port();
    std::fs::write(&endpoint, format!("http://127.0.0.1:{port}/hook")).expect("endpoint");
    let auth = endpoint.with_file_name("hook-auth");
    std::fs::write(
        &auth,
        format!("{}: a-test-secret\n", devpit_agentcli::HOOK_HEADER),
    )
    .expect("secret");

    let served = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).expect("blocking");
                    let posted = read_request(&mut stream);
                    reply(&mut stream, "");
                    return posted;
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => return None,
            }
        }
    });

    let command = command_for(&devpit_agentcli::settings_json(&endpoint, &auth), event);
    let mut hook = Command::new("sh");
    hook.arg("-c")
        .arg(&command)
        .env_remove("DEVPIT_PANE")
        .stdin(Stdio::piped());
    if let Some(pane) = pane {
        hook.env("DEVPIT_PANE", pane);
    }
    let mut child = hook.spawn().expect("hook");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(payload.as_bytes())
        .expect("payload");
    child.wait().expect("hook ran");
    served
        .join()
        .expect("served")
        .unwrap_or_else(|| panic!("the {event} hook never posted"))
}

/// Every way a hook reaches a card, each one fired through the real hook
/// command: the pane route, a run's session, a background session, a post that
/// arrives out of order, and a session ending.
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
    let step = store
        .create_step(&project, "agent", "review", "{}", false)
        .expect("step");
    let run = store.start_run(&card, &step, None).expect("run");
    store.set_run_session(&run, "s-run").expect("run session");
    store
        .link_session(&card, "a1b2", "s-bg", None, None, None)
        .expect("background");
    drop(store);

    let sink = Recorded {
        store_path,
        activities: Mutex::default(),
        said: Mutex::default(),
    };
    let state_of = |kind: SessionKind, reference: &str| {
        sink.activities
            .lock()
            .expect("activities")
            .happening(&card)
            .sessions
            .iter()
            .find(|one| one.kind == kind && one.reference == reference)
            .and_then(|one| one.state)
    };

    // The pane route: the agent in the card's pane stops.
    let stopped = posted_by_hook(
        dir.path(),
        "Stop",
        r#"{"hook_event_name":"Stop","session_id":"s-pane","cwd":"/w"}"#,
        Some(LEAF),
    );
    hear_post(&sink, &stopped, 10);
    assert_eq!(state_of(SessionKind::Pane, LEAF), Some(Doing::Done));
    {
        let said = sink.said.lock().expect("said");
        let (_, happening) = said
            .iter()
            .rev()
            .find(|(channel, _)| channel == "card:happening")
            .expect("the card heard it");
        assert_eq!(happening["cardId"], card.as_str());
        assert_eq!(happening["sessions"][0]["ref"], LEAF);
        assert_eq!(
            happening["sessions"][0]["tabId"],
            tab_for_card(&card).as_str()
        );
    }

    // A run's session id, from a headless turn with no pane.
    let used = posted_by_hook(
        dir.path(),
        "PostToolUse",
        r#"{"hook_event_name":"PostToolUse","tool_name":"Edit","session_id":"s-run","cwd":"/w"}"#,
        None,
    );
    assert_eq!(used.pane, None);
    hear_post(&sink, &used, 11);
    assert_eq!(state_of(SessionKind::Run, "s-run"), Some(Doing::Working));

    // A background session's id.
    let asked = posted_by_hook(
        dir.path(),
        "Notification",
        r#"{"hook_event_name":"Notification","session_id":"s-bg","cwd":"/w","message":"needs you"}"#,
        None,
    );
    hear_post(&sink, &asked, 12);
    assert_eq!(
        state_of(SessionKind::Background, "s-bg"),
        Some(Doing::Waiting)
    );

    // Out of order: a post stamped before the stop, served after it.
    let late = posted_by_hook(
        dir.path(),
        "Notification",
        r#"{"hook_event_name":"Notification","session_id":"s-pane","cwd":"/w","message":"needs you"}"#,
        Some(LEAF),
    );
    hear_post(&sink, &late, 9);
    assert_eq!(state_of(SessionKind::Pane, LEAF), Some(Doing::Done));

    // SessionEnded: the pane's session ends.
    let ended = posted_by_hook(
        dir.path(),
        "SessionEnd",
        r#"{"hook_event_name":"SessionEnd","session_id":"s-pane","cwd":"/w","reason":"prompt_input_exit"}"#,
        Some(LEAF),
    );
    hear_post(&sink, &ended, 13);
    assert_eq!(state_of(SessionKind::Pane, LEAF), Some(Doing::Gone));
    assert_eq!(
        sink.activities
            .lock()
            .expect("activities")
            .happening(&card)
            .activity,
        Some(Doing::Waiting)
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
        .link_session(&card, "a1b2", "s-bg", None, Some("/w/card"), None)
        .expect("link");
    let sink = Recorded {
        store_path,
        activities: Mutex::default(),
        said: Mutex::default(),
    };

    let posted = Posted {
        body: BACKGROUND_WAITING.to_owned(),
        pane: None,
        secret: None,
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
        secret: None,
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

/// Only this run's secret opens the door.
///
/// The port is visible to anything on the machine that can list sockets; the
/// secret is in a file only its owner can read. Length is checked first and
/// then every byte is compared, so a caller learns nothing from how long the
/// refusal took.
#[test]
fn only_the_secret_this_run_made_is_heard() {
    assert!(authorized(Some("6f1c"), "6f1c"));
    assert!(!authorized(None, "6f1c"), "a post with no header was heard");
    assert!(!authorized(Some(""), "6f1c"));
    assert!(!authorized(Some("6f1d"), "6f1c"));
    assert!(
        !authorized(Some("6f1c00"), "6f1c"),
        "a longer guess was heard"
    );
}
