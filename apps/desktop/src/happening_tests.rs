use devpit_agentcli::Event;

use super::*;

fn detail(event: &Event) -> serde_json::Value {
    let said = subagent_said("leaf_1", event).expect("a subagent happening");
    assert_eq!(
        (said.pane_id.as_str(), said.what.as_str()),
        ("leaf_1", "subagent")
    );
    serde_json::from_str(&said.detail.expect("a detail")).expect("json")
}

/// Start, launch and stop all land on the same row, which is found by its id.
#[test]
fn a_subagent_reaches_its_pane_as_one_row_by_id() {
    let id = "a82d987eefe13ba95".to_owned();
    let started = detail(&Event::SubagentStarted {
        agent: id.clone(),
        kind: Some("general-purpose".to_owned()),
    });
    assert_eq!(started["id"], "a82d987eefe13ba95");
    assert_eq!(started["kind"], "general-purpose");

    let named = detail(&Event::Delegated {
        agent: id.clone(),
        description: Some("probe child".to_owned()),
        model: Some("claude-haiku-4-5-20251001".to_owned()),
        ended: false,
    });
    assert_eq!(
        (&named["id"], &named["name"], &named["ended"]),
        (&started["id"], &"probe child".into(), &false.into())
    );

    let stopped = detail(&Event::SubagentDone { agent: Some(id) });
    assert_eq!(
        (&stopped["id"], &stopped["ended"]),
        (&started["id"], &true.into())
    );
}

/// A stop with no id cannot end a row, and the agent's own events are not rows.
#[test]
fn what_names_no_subagent_is_not_a_subagent() {
    for event in [
        Event::SubagentDone { agent: None },
        Event::Waiting,
        Event::Using {
            tool: "Agent".to_owned(),
        },
    ] {
        assert!(subagent_said("leaf_1", &event).is_none(), "{event:?}");
    }
}

fn heard(transcript: Option<&str>, session: &str) -> devpit_agentcli::Happening {
    devpit_agentcli::Happening {
        session_id: session.to_owned(),
        event: Event::Waiting,
        cwd: "/work".to_owned(),
        transcript_path: transcript.map(str::to_owned),
    }
}

#[test]
fn a_pane_learns_which_session_its_agent_is_in() {
    let transcript = "/home/me/.claude-glm/projects/-work/abc.jsonl";
    let said = session_said("leaf_1", &heard(Some(transcript), "abc")).expect("a session");
    assert_eq!(
        (said.pane_id.as_str(), said.what.as_str()),
        ("leaf_1", "session")
    );
    let detail: serde_json::Value =
        serde_json::from_str(&said.detail.expect("detail")).expect("json");
    assert_eq!(detail["sessionId"], "abc");
    assert_eq!(detail["transcript"], transcript);
}

/// Without both halves there is nothing a chat could resume.
#[test]
fn a_pane_learns_nothing_from_half_a_session() {
    assert!(session_said("leaf_1", &heard(None, "abc")).is_none());
    assert!(session_said("leaf_1", &heard(Some("/t.jsonl"), "")).is_none());
}
