use std::cell::RefCell;

use serde_json::json;

use super::*;

/// Sessions that only say they started, and move the card the way
/// `card_follows::start` would.
#[derive(Default)]
struct Fake {
    started: RefCell<Vec<(String, String)>>,
}

impl Sessions for Fake {
    fn start(&self, _board: &Board, card_id: &str, prompt: &str) -> Result<(), String> {
        self.started
            .borrow_mut()
            .push((card_id.to_owned(), prompt.to_owned()));
        Ok(())
    }
}

const LANES: [(&str, &str); 4] = [
    ("col_backlog", "backlog"),
    ("col_doing", "doing"),
    ("col_check", "check"),
    ("col_done", "done"),
];

fn card(
    id: &str,
    title: &str,
    lane: &str,
    parent: Option<&str>,
    blocked_by: &[(&str, bool)],
) -> Value {
    json!({
        "id": id, "columnId": lane, "title": title, "body": "", "position": 0,
        "worktreePath": null, "dueAt": null, "dueTime": false, "costUsd": 0.0,
        "comments": 0, "pinned": 0, "runs": [], "activity": null, "parentId": parent,
        "blockedBy": blocked_by.iter().map(|(by, done)| json!({ "id": by, "title": by, "done": done })).collect::<Vec<_>>(),
    })
}

/// The epic and its three cards, each waiting on the one before, in their lanes.
fn board(lanes: [&str; 3]) -> Board {
    let done = |lane: &str| lane == "col_done";
    serde_json::from_value(json!({
        "projectId": "prj_auto",
        "steps": [],
        "columns": LANES.iter().enumerate().map(|(at, (id, role))| json!({
            "id": id, "name": role, "position": at, "step": null, "onPass": null,
            "autonomy": "manual", "role": role,
        })).collect::<Vec<_>>(),
        "cards": [
            card("epic", "[devflow] Brief: Search", "col_backlog", None, &[]),
            card("c", "[devflow] Keep the query", lanes[2], Some("epic"), &[("b", done(lanes[1]))]),
            card("a", "[devflow] Index the bodies", lanes[0], Some("epic"), &[]),
            card("b", "[devflow] List what is found", lanes[1], Some("epic"), &[("a", done(lanes[0]))]),
        ],
    }))
    .expect("a board")
}

fn step(lanes: [&str; 3], sessions: &Fake) -> Next {
    stepped(&board(lanes), "epic", false, sessions)
        .expect("stepped")
        .0
}

#[test]
fn the_plans_cards_start_one_after_another_and_stop_at_the_approval_point() {
    let sessions = Fake::default();
    let waiting = "col_backlog";

    assert_eq!(
        step([waiting, waiting, waiting], &sessions),
        Next::Start("a".into())
    );
    let (card, prompt) = sessions.started.borrow()[0].clone();
    assert_eq!(card, "a");
    assert!(
        prompt.contains("devpit_devflow_gate") && prompt.contains("devpit_handoff"),
        "{prompt}"
    );
    assert!(
        prompt.contains("before any commit, push or deploy"),
        "{prompt}"
    );

    // At work: nothing else starts.
    assert_eq!(
        step(["col_doing", waiting, waiting], &sessions),
        Next::Working("a".into())
    );
    // Finished, and b waits on it: the person checks it first.
    assert!(matches!(
        step(["col_check", waiting, waiting], &sessions),
        Next::Approve { card, .. } if card == "a"
    ));
    assert_eq!(
        sessions.started.borrow().len(),
        1,
        "it went past the approval point"
    );
    // The person moved it to done: b starts, then c once b is done.
    assert_eq!(
        step(["col_done", waiting, waiting], &sessions),
        Next::Start("b".into())
    );
    assert_eq!(
        step(["col_done", "col_done", waiting], &sessions),
        Next::Start("c".into())
    );
    assert_eq!(
        step(["col_done", "col_done", "col_check"], &sessions),
        Next::Done
    );
    let order: Vec<String> = sessions
        .started
        .borrow()
        .iter()
        .map(|(card, _)| card.clone())
        .collect();
    assert_eq!(order, ["a", "b", "c"]);
}

#[test]
fn a_session_waiting_on_the_person_stops_the_autopilot() {
    let sessions = Fake::default();
    let mut stuck = board(["col_doing", "col_backlog", "col_backlog"]);
    if let Some(card) = stuck.cards.iter_mut().find(|card| card.id == "a") {
        card.activity = Some(Doing::Waiting);
    }
    let (next, said) = stepped(&stuck, "epic", false, &sessions).expect("stepped");
    assert!(matches!(next, Next::Blocked { .. }), "{next:?}");
    assert!(said.contains("waits for your answer"), "{said}");
    assert!(sessions.started.borrow().is_empty());
}

#[test]
fn a_flow_with_an_approval_after_the_plan_stops_after_each_card() {
    let approving = devpit_devflow::config::parse(
        "version: 2\nstages:\n  - uses: builtin/plan\n  - uses: builtin/execution\n  - uses: project/release\n    approve: person\n",
    )
    .expect("reads");
    assert!(approve_each(&approving));
    let plain = devpit_devflow::config::parse("version: 2\n").expect("reads");
    assert!(!approve_each(&plain));
    let sessions = Fake::default();
    let (next, _) = stepped(
        &board(["col_check", "col_backlog", "col_backlog"]),
        "epic",
        approve_each(&approving),
        &sessions,
    )
    .expect("stepped");
    assert!(matches!(next, Next::Approve { card, .. } if card == "a"));
}
