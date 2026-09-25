use std::path::Path;

use super::{events, logs_of, woken_by, PeerEvent};

const LOG: &str = r#"{"type":"assistant","timestamp":"2026-09-25T18:11:55Z","message":{"content":[{"type":"tool_use","name":"SendMessage","input":{"to":"gatorclaw-82","summary":"Build and deploy dev","message":"Update master and deploy."}}]}}
{"type":"user","isMeta":true,"timestamp":"2026-09-25T18:13:45Z","origin":{"kind":"peer","name":"gatorclaw-82","body":"Build ready, not deployed."},"message":{"content":"Another Claude session sent a message: ..."}}
{"type":"user","isMeta":true,"timestamp":"2026-09-25T18:13:56Z","message":{"content":"[Cross-session idle notice] \"gatorclaw-82\", which you asked to be notified about, is idle now. Its harness reports: «The build is ready…». This is an automated notice."}}
{"type":"user","timestamp":"2026-09-25T18:14:00Z","message":{"content":"thanks"}}
{"type":"assistant","timestamp":"2026-09-25T18:14:01Z","message":{"content":[{"type":"text","text":"SendMessage is a tool I have"}]}}"#;

#[test]
fn what_passed_between_sessions_is_read_in_order() {
    let found = events(LOG);
    assert_eq!(found.len(), 3);
    assert_eq!(
        found[0],
        PeerEvent::Sent {
            at: "2026-09-25T18:11:55Z".into(),
            to: "gatorclaw-82".into(),
            summary: "Build and deploy dev".into(),
            message: "Update master and deploy.".into(),
        }
    );
    assert_eq!(
        found[1],
        PeerEvent::Heard {
            at: "2026-09-25T18:13:45Z".into(),
            from: "gatorclaw-82".into(),
            body: "Build ready, not deployed.".into(),
        }
    );
    assert_eq!(
        found[2],
        PeerEvent::Idle {
            at: "2026-09-25T18:13:56Z".into(),
            from: "gatorclaw-82".into(),
            said: "The build is ready…".into(),
        }
    );
    assert!(found.iter().all(|one| one.peer() == "gatorclaw-82"));
}

#[test]
fn a_folders_logs_are_where_the_cli_keeps_them() {
    assert_eq!(
        logs_of(
            Path::new("/home/me/.claude"),
            Path::new("/home/me/.devpit/orchestrator/work")
        ),
        Path::new("/home/me/.claude/projects/-home-me--devpit-orchestrator-work")
    );
}

#[test]
fn a_turn_is_woken_by_the_last_thing_said_to_it_if_a_session_said_it() {
    // The person spoke last: nobody else woke it.
    assert_eq!(woken_by(LOG), None);
    let idle_last = LOG.lines().take(3).collect::<Vec<_>>().join("\n");
    assert!(matches!(woken_by(&idle_last), Some(PeerEvent::Idle { .. })));
    let heard_last = LOG.lines().take(2).collect::<Vec<_>>().join("\n");
    assert!(matches!(
        woken_by(&heard_last),
        Some(PeerEvent::Heard { .. })
    ));
}
