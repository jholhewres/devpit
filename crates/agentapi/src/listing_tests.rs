use serde_json::{json, Value};

use super::*;
use crate::devflow::{Offer, Watch, SETUP_TOOLS, STAGED};
use crate::mcp::{handle_offering, TOOLS};

fn never(_: &str, _: Value) -> Result<Value, String> {
    panic!("the app should not have been asked")
}

fn tools(offer: &Offer, seat: &Seat) -> Vec<Value> {
    let said: Value = serde_json::from_str(
        &handle_offering(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#,
            &never,
            true,
            offer,
            seat,
        )
        .expect("an answer"),
    )
    .expect("json");
    said["result"]["tools"].as_array().expect("tools").clone()
}

fn names(listed: &[Value]) -> Vec<&str> {
    listed
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default())
        .collect()
}

const PROJECT: Seat = Seat {
    orchestrator: Some(false),
    decisions: Some(false),
};

#[test]
fn every_tool_named_here_is_one_devpit_has() {
    let every: Vec<&str> = TOOLS
        .iter()
        .chain(&SETUP_TOOLS)
        .chain(STAGED.iter().map(|(_, tool)| tool))
        .map(|tool| tool.name)
        .collect();
    for name in ORCHESTRATOR_ONLY
        .iter()
        .chain(&DECISIONS)
        .chain(&READ_ONLY)
        .chain(&DESTRUCTIVE)
    {
        assert!(every.contains(name), "{name}");
    }
}

/// A host approves a read without asking, and asks before what it cannot undo.
#[test]
fn a_read_and_a_loss_say_so_to_the_host() {
    let listed = tools(&Offer::Active(vec!["plan".into()]), &Seat::default());
    let hint = |name: &str| {
        listed
            .iter()
            .find(|tool| tool["name"] == name)
            .map(|tool| tool["annotations"].clone())
            .expect(name)
    };
    assert_eq!(hint("devpit_board"), json!({ "readOnlyHint": true }));
    assert_eq!(
        hint("devpit_devflow_skill"),
        json!({ "readOnlyHint": true })
    );
    assert_eq!(
        hint("devpit_stop_session"),
        json!({ "destructiveHint": true })
    );
    assert_eq!(hint("devpit_comment"), Value::Null);
}

/// An app that does not say who the session is gets the list it always had.
#[test]
fn an_app_too_old_to_say_lists_everything() {
    let all = tools(&Offer::Setup, &Seat::default());
    assert_eq!(all.len(), TOOLS.len() + SETUP_TOOLS.len());
    let board = all
        .iter()
        .find(|tool| tool["name"] == "devpit_board")
        .expect("the board");
    assert!(board["inputSchema"]["properties"]["project"].is_object());
}

#[test]
fn a_project_session_is_offered_neither_the_orchestrators_tools_nor_another_board() {
    let listed = tools(&Offer::Setup, &PROJECT);
    let names = names(&listed);
    for name in ORCHESTRATOR_ONLY.iter().chain(&DECISIONS) {
        assert!(!names.contains(name), "{name} was listed");
    }
    for name in ["devpit_board", "devpit_card", "devpit_devflow_status"] {
        assert!(names.contains(&name), "{name} was not listed");
    }
    for tool in &listed {
        assert!(
            tool["inputSchema"]["properties"].get("project").is_none(),
            "{} still takes another project",
            tool["name"]
        );
    }
}

#[test]
fn an_orchestrator_is_offered_its_own_tools_and_another_projects_board() {
    let seat = Seat {
        orchestrator: Some(true),
        decisions: Some(true),
    };
    // The app answers `off` for an orchestrator: its folder is notes.
    let listed = tools(&Offer::Off, &seat);
    let names = names(&listed);
    for name in ORCHESTRATOR_ONLY.iter().chain(&DECISIONS) {
        assert!(names.contains(name), "{name} was not listed");
    }
    // Of Devflow, only what passes the person's yes on to a linked project.
    let devflow: Vec<&&str> = names
        .iter()
        .filter(|name| name.starts_with("devpit_devflow"))
        .collect();
    assert_eq!(devflow, [&"devpit_devflow_plan"]);
    let board = listed
        .iter()
        .find(|tool| tool["name"] == "devpit_board")
        .expect("the board");
    assert!(board["inputSchema"]["properties"]["project"].is_object());
}

#[test]
fn an_active_project_lists_its_stages_but_not_what_it_cannot_use() {
    let listed = tools(&Offer::Active(vec!["plan".into()]), &PROJECT);
    let active = names(&listed);
    assert!(active.contains(&"devpit_devflow_plan"));
    assert!(!active.contains(&"devpit_start_session"));
    assert!(!active.contains(&"devpit_decide"));
    let with_key = Seat {
        decisions: Some(true),
        ..PROJECT
    };
    let keyed = tools(&Offer::Off, &with_key);
    assert!(names(&keyed).contains(&"devpit_decide"));
}

/// Off the list is not out of reach: a call by name still asks the app,
/// which refuses or answers as it always did.
#[test]
fn a_tool_off_the_list_is_still_answered_by_name() {
    let ask = |method: &str, _: Value| -> Result<Value, String> {
        assert_eq!(method, "sessions");
        Err("only an orchestrator sees past its own project".into())
    };
    let said: Value = serde_json::from_str(
        &handle_offering(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"devpit_sessions"}}"#,
            &ask,
            true,
            &Offer::Off,
            &PROJECT,
        )
        .expect("an answer"),
    )
    .expect("json");
    assert_eq!(said["result"]["isError"], true);
    assert!(said["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .contains("orchestrator"));
}

#[test]
fn a_key_kept_or_forgotten_tells_the_client_to_list_again() {
    let root = tempfile::tempdir().expect("root");
    let mut watch = Watch::new(root.path());
    let looked = watch.due().expect("due");
    let without = json!({ "offer": "setup", "orchestrator": false, "decisions": false });
    assert!(watch.settle(looked.clone(), Ok(without.clone())).is_some());
    assert_eq!(watch.seat, PROJECT);
    assert!(watch.settle(looked.clone(), Ok(without)).is_none());
    let with = json!({ "offer": "setup", "orchestrator": false, "decisions": true });
    assert!(watch.settle(looked, Ok(with)).is_some());
    assert_eq!(watch.seat.decisions, Some(true));
}
