use devpit_agentcli::{Event, Happening};
use std::collections::{HashMap, HashSet};

use devpit_rpc::{Doing, Touch};

use super::*;

fn heard(event: Event) -> Happening {
    Happening {
        session_id: "s1".to_owned(),
        event,
        cwd: "/w/app".to_owned(),
        transcript_path: None,
    }
}

fn using(tool: &str, target: &str) -> Event {
    Event::Using {
        tool: tool.to_owned(),
        target: Some(target.to_owned()),
        touch: None,
    }
}

/// Folds events in order, as the hooks would deliver them.
fn folded(events: Vec<Event>) -> Option<IslandSession> {
    events
        .into_iter()
        .enumerate()
        .fold(None, |was, (at, event)| {
            fold(was, Some("leaf_1"), &heard(event), None, at as f64)
        })
}

#[test]
fn a_step_is_kept_with_what_it_ran_on_until_it_is_done() {
    let session = folded(vec![Event::Prompted, using("Edit", "invoice.ts")]).expect("a session");
    assert_eq!(session.state, Doing::Working);
    assert_eq!(session.steps.len(), 1);
    assert_eq!(session.steps[0].target.as_deref(), Some("invoice.ts"));
    assert!(!session.steps[0].done);

    let session = folded(vec![
        Event::Prompted,
        using("Edit", "invoice.ts"),
        Event::Used {
            tool: "Edit".to_owned(),
        },
    ])
    .expect("a session");
    assert!(session.steps[0].done);
}

/// Working with no step yet is the agent thinking, which the island draws: a
/// prompt starts a turn over, and last turn's steps would say otherwise.
#[test]
fn a_new_prompt_starts_the_turn_with_no_steps() {
    let session = folded(vec![
        Event::Prompted,
        using("Read", "a.rs"),
        Event::Stopped {
            said: Some("Done.".to_owned()),
        },
        Event::Prompted,
    ])
    .expect("a session");
    assert_eq!(session.state, Doing::Working);
    assert!(session.steps.is_empty());
    assert_eq!(session.said, None);
}

#[test]
fn a_turn_that_stops_ends_its_steps_and_says_what_it_said() {
    let session = folded(vec![
        Event::Prompted,
        using("Bash", "npm test"),
        Event::Stopped {
            said: Some("All green.".to_owned()),
        },
    ])
    .expect("a session");
    assert_eq!(session.state, Doing::Done);
    assert!(session.steps.iter().all(|step| step.done));
    assert_eq!(session.said.as_deref(), Some("All green."));
}

#[test]
fn only_the_latest_steps_are_kept() {
    let mut events = vec![Event::Prompted];
    events.extend((0..STEPS + 3).map(|n| using("Read", &format!("{n}.rs"))));
    let session = folded(events).expect("a session");
    assert_eq!(session.steps.len(), STEPS);
    assert_eq!(
        session.steps.last().and_then(|step| step.target.as_deref()),
        Some(format!("{}.rs", STEPS + 2).as_str())
    );
}

#[test]
fn an_ended_session_leaves_the_island_but_a_clear_does_not() {
    assert!(folded(vec![
        Event::Prompted,
        Event::SessionEnded {
            reason: Some("prompt_input_exit".to_owned())
        },
    ])
    .is_none());

    let cleared = folded(vec![
        Event::Prompted,
        using("Read", "a.rs"),
        Event::SessionEnded {
            reason: Some("clear".to_owned()),
        },
    ])
    .expect("still there");
    assert!(cleared.steps.is_empty());
}

#[test]
fn waiting_on_a_person_is_kept_as_the_state() {
    let session = folded(vec![Event::Prompted, Event::Waiting]).expect("a session");
    assert_eq!(session.state, Doing::Waiting);
}

#[test]
fn where_it_was_placed_is_written_onto_the_session() {
    let placed = Placed {
        project_id: Some("prj_1".to_owned()),
        project: Some("api".to_owned()),
        color: Some("#e2795b".to_owned()),
        card_id: Some("card_1".to_owned()),
        card: Some("Fix the invoice".to_owned()),
        root: Some("/w/app".to_owned()),
    };
    let session = fold(
        None,
        Some("leaf_1"),
        &heard(Event::Prompted),
        Some(placed),
        1.0,
    )
    .expect("a session");
    assert_eq!(session.project.as_deref(), Some("api"));
    assert_eq!(session.card.as_deref(), Some("Fix the invoice"));
    assert_eq!(session.pane_id.as_deref(), Some("leaf_1"));
}

#[test]
fn the_touch_travels_with_its_step() {
    let edit = Event::Using {
        tool: "Bash".to_owned(),
        target: Some("ls".to_owned()),
        touch: Some(Touch::Run {
            command: "ls".to_owned(),
        }),
    };
    let session = folded(vec![Event::Prompted, edit]).expect("a session");
    assert!(matches!(session.steps[0].touch, Some(Touch::Run { .. })));
}

#[test]
fn a_session_belongs_to_the_deepest_project_holding_its_folder() {
    let projects = [
        ("outer", "/w"),
        ("inner", "/w/app"),
        ("other", "/w/application"),
    ];
    let under = |cwd: &str| project_under(projects.iter().copied(), cwd);
    assert_eq!(under("/w/app/src").as_deref(), Some("inner"));
    assert_eq!(under("/w/app").as_deref(), Some("inner"));
    assert_eq!(under("/w/apple").as_deref(), Some("outer"));
    assert_eq!(under("/elsewhere"), None);
}

#[test]
fn a_card_checkout_belongs_to_its_project() {
    let projects = [("prj_1", "/w/app")];
    assert_eq!(
        project_under(
            projects.iter().copied(),
            "/home/me/.devpit/worktrees/prj_1/card_9/src"
        )
        .as_deref(),
        Some("prj_1")
    );
}

#[test]
fn a_step_whose_tool_failed_says_so() {
    let session = folded(vec![
        Event::Prompted,
        using("Bash", "npm test"),
        Event::UseFailed {
            tool: "Bash".to_owned(),
        },
    ])
    .expect("a session");
    assert!(session.steps[0].done);
    assert!(session.steps[0].failed);
}

fn listed(session_id: Option<&str>, status: &str, in_pane: bool) -> devpit_rpc::LiveSession {
    devpit_rpc::LiveSession {
        name: "anchored-1a".to_owned(),
        pid: 1,
        job: None,
        status: status.to_owned(),
        kind: "interactive".to_owned(),
        cwd: "/w/anchored".to_owned(),
        project_id: Some("prj_a".to_owned()),
        project_name: Some("anchored".to_owned()),
        card_id: None,
        since: Some(5.0),
        in_devpit: in_pane,
        waiting: None,
        pane: in_pane.then(|| devpit_rpc::LivePane {
            project_id: "prj_a".to_owned(),
            pane_id: "leaf_1".to_owned(),
        }),
        session_id: session_id.map(str::to_owned),
        step: None,
        draft: None,
    }
}

/// An idle session left from before a restart says nothing until it is used;
/// the CLI still lists it, and so does the island.
#[test]
fn a_live_session_no_hook_spoke_for_is_brought_in() {
    let mut sessions = Sessions::new();
    let colors = HashMap::from([("prj_a".to_owned(), Some("#62c987".to_owned()))]);
    seed_into(
        &mut sessions,
        vec![
            listed(Some("s1"), "idle", true),
            listed(Some("s2"), "busy", true),
            listed(Some("s3"), "busy", false),
        ],
        &colors,
        &open(&["leaf_1"]),
    );
    assert_eq!(sessions["s1"].state, Doing::Open);
    assert_eq!(sessions["s2"].state, Doing::Working);
    assert_eq!(sessions["s1"].color.as_deref(), Some("#62c987"));
    assert!(!sessions.contains_key("s3"), "not in a devpit terminal");

    let mut elsewhere = listed(Some("s4"), "busy", true);
    elsewhere.project_id = None;
    seed_into(&mut sessions, vec![elsewhere], &colors, &open(&["leaf_1"]));
    assert!(
        !sessions.contains_key("s4"),
        "a project this devpit does not know"
    );
}

#[test]
fn what_a_hook_said_is_not_overwritten_by_the_listing() {
    let mut sessions = Sessions::new();
    let heard = fold(None, Some("leaf_1"), &heard(Event::Waiting), None, 9.0).expect("a session");
    sessions.insert("s1".to_owned(), heard);
    seed_into(
        &mut sessions,
        vec![listed(Some("s1"), "idle", true)],
        &HashMap::new(),
        &open(&["leaf_1"]),
    );
    assert_eq!(sessions["s1"].state, Doing::Waiting);
}

fn open(panes: &[&str]) -> HashSet<String> {
    panes.iter().map(|one| (*one).to_owned()).collect()
}

/// The case that was reported: the CLI listed a session in a pane named like
/// devpit's — a development build's, on its own tmux server — in a folder this
/// devpit knows, and the island showed a terminal nobody had open here.
#[test]
fn a_session_in_a_terminal_not_open_here_is_not_brought_in() {
    let mut sessions = Sessions::new();
    seed_into(
        &mut sessions,
        vec![listed(Some("s1"), "idle", true)],
        &HashMap::new(),
        &open(&["leaf_other"]),
    );
    assert!(sessions.is_empty());
}

fn in_pane(id: &str, pane: Option<&str>, state: Doing, at: f64) -> IslandSession {
    let mut session = folded(vec![Event::Prompted]).expect("a session");
    session.session_id = id.to_owned();
    session.pane_id = pane.map(str::to_owned);
    session.state = state;
    session.at = at;
    session
}

/// What is open in devpit decides, not what a hook last said: a terminal
/// closed or back at its shell takes its session with it, and a terminal
/// holds one agent — an older session left there by a `/clear` goes.
#[test]
fn the_island_keeps_what_is_open_in_devpit() {
    let mut sessions: Sessions = [
        in_pane("open", Some("leaf_1"), Doing::Open, 1.0),
        in_pane("closed", Some("leaf_2"), Doing::Working, 2.0),
        in_pane("before-clear", Some("leaf_3"), Doing::Open, 1.0),
        in_pane("after-clear", Some("leaf_3"), Doing::Working, 5.0),
        in_pane("chat", None, Doing::Open, 1.0),
    ]
    .into_iter()
    .map(|one| (one.session_id.clone(), one))
    .collect();

    let mut gone = reconcile(&mut sessions, &open(&["leaf_1", "leaf_3"]));
    gone.sort();
    assert_eq!(gone, ["before-clear", "closed"]);
    let mut kept: Vec<&str> = sessions.keys().map(String::as_str).collect();
    kept.sort();
    assert_eq!(kept, ["after-clear", "chat", "open"]);
}

/// An idle session in an open terminal stays, however long it has been idle;
/// one with no terminal — a chat — still rests after two hours.
#[test]
fn an_idle_terminal_session_is_not_rested_away() {
    let sessions: Sessions = [
        in_pane("terminal", Some("leaf_1"), Doing::Open, 0.0),
        in_pane("chat", None, Doing::Open, 0.0),
    ]
    .into_iter()
    .map(|one| (one.session_id.clone(), one))
    .collect();
    let ten_hours = 10.0 * 3600.0 * 1000.0;
    assert_eq!(resting(&sessions, ten_hours), ["chat"]);
}

/// A row says what a working session is on, the newest step first, and says
/// nothing once the turn is over.
#[test]
fn a_working_session_says_its_newest_step() {
    let mut session = folded(vec![
        Event::Prompted,
        using("Read", "a.rs"),
        using("Edit", "invoice.ts"),
        Event::Used {
            tool: "Edit".to_owned(),
        },
    ])
    .expect("a session");
    assert_eq!(stepping(&session).as_deref(), Some("Edit invoice.ts"));

    session.steps.push(IslandStep {
        tool: "TodoWrite".to_owned(),
        target: None,
        done: false,
        failed: false,
        touch: None,
    });
    assert_eq!(stepping(&session).as_deref(), Some("TodoWrite"));

    let stopped = folded(vec![
        Event::Prompted,
        using("Edit", "invoice.ts"),
        Event::Stopped { said: None },
    ])
    .expect("a session");
    assert_eq!(stepping(&stopped), None);
    let thinking = folded(vec![Event::Prompted]).expect("a session");
    assert_eq!(stepping(&thinking), None, "a turn that touched nothing yet");
}
