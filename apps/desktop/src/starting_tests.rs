use std::time::{Duration, Instant};

use devpit_rpc::LiveSession;

use super::{shown, Starting};

const TRUST: &str = "\
 Do you trust the files in this folder?

 /home/ana/work/interface

 ❯ 1. Yes, proceed
   2. No, exit

 Enter to confirm · Esc to exit
";

fn begun(name: &str, at: Instant) -> Starting {
    Starting {
        profile: "claude".to_owned(),
        name: name.to_owned(),
        project_id: "prj_ui".to_owned(),
        project_name: Some("interface".to_owned()),
        cwd: "/home/ana/work/interface".to_owned(),
        pane_id: "leaf_9".to_owned(),
        at,
    }
}

fn listed(name: &str) -> LiveSession {
    serde_json::from_value(serde_json::json!({
        "name": name, "pid": 1, "job": null, "status": "idle", "kind": "interactive",
        "cwd": "/w", "projectId": null, "projectName": null, "cardId": null, "since": null,
        "inDevpit": true, "waiting": null, "pane": null, "sessionId": null, "step": null,
        "draft": null,
    }))
    .expect("a session")
}

/// The case that started this: a session handed to a new project stops on
/// the folder's trust question before the CLI lists it, and the question is
/// shown where every other one is.
#[test]
fn a_session_stopped_before_it_is_listed_shows_its_question() {
    let now = Instant::now();
    let mut held = vec![begun("interface-c7tf", now)];
    let rows = shown(&mut held, "claude", &[], now, |_| Some(TRUST.to_owned()));
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row.name, "interface-c7tf");
    assert_eq!(row.status, "starting");
    let waiting = row.waiting.as_ref().expect("its question");
    assert!(
        waiting.question.contains("trust the files"),
        "{}",
        waiting.question
    );
    assert_eq!(waiting.options.len(), 2);
    assert_eq!(
        row.pane.as_ref().map(|pane| pane.pane_id.as_str()),
        Some("leaf_9")
    );
}

/// Nothing to show while it is only starting, and it is let go once the CLI
/// lists it, or once it has been gone too long — and another account's are
/// neither shown nor let go.
#[test]
fn a_session_is_let_go_once_listed_or_long_gone() {
    let now = Instant::now();
    let mut held = vec![begun("quiet", now)];
    assert!(shown(&mut held, "claude", &[], now, |_| Some(
        "Starting…".to_owned()
    ))
    .is_empty());
    assert_eq!(held.len(), 1, "still looked for");

    shown(&mut held, "claude", &[listed("quiet")], now, |_| {
        Some(TRUST.to_owned())
    });
    assert!(held.is_empty(), "listed by the CLI: its own row now");

    let mut held = vec![begun("old", now - Duration::from_secs(11 * 60))];
    assert!(shown(&mut held, "claude", &[], now, |_| Some(TRUST.to_owned())).is_empty());
    assert!(held.is_empty());

    let mut held = vec![begun("theirs", now)];
    assert!(shown(&mut held, "claudin", &[], now, |_| Some(TRUST.to_owned())).is_empty());
    assert_eq!(
        held.len(),
        1,
        "another account's session is not this one's to let go"
    );
}
