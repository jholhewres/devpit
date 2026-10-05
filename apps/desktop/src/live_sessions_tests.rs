use std::collections::HashMap;
use std::path::Path;

use devpit_rpc::Project;
use serde_json::json;

use super::{card_of, pane_of, pane_target, read, start_of};

/// tmux's answer to which window each pane is in.
fn windows(panes: &[(&str, &str)]) -> HashMap<String, String> {
    panes
        .iter()
        .map(|(pane, window)| ((*pane).to_owned(), (*window).to_owned()))
        .collect()
}

fn project(id: &str, root: &Path, orchestrator: Option<&str>) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": orchestrator,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

fn listing(dir: &Path, pid: i32, name: &str, cwd: &Path) {
    let body = json!({ "pid": pid, "name": name, "status": "busy", "kind": "interactive",
        "cwd": cwd.display().to_string(), "statusUpdatedAt": 1.0 });
    std::fs::write(dir.join(format!("{pid}.json")), body.to_string()).expect("listing");
    // The CLI's key file beside it is not a listing.
    std::fs::write(dir.join(format!("{pid}.abc.key")), "secret").expect("key");
}

#[test]
fn a_live_session_is_placed_on_its_project_and_card() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sessions = dir.path().join("sessions");
    let app = dir.path().join("app");
    let worktrees = dir.path().join("worktrees");
    let checkout = worktrees.join("prj_1").join("card_7");
    for path in [&sessions, &app, &checkout] {
        std::fs::create_dir_all(path).expect("mkdir");
    }
    listing(&sessions, 10, "api-worker", &app);
    listing(&sessions, 11, "card-worker", &checkout);

    let found = read(
        &sessions,
        |_| true,
        &[project("prj_1", &app, None)],
        &worktrees,
        &HashMap::new(),
        |_| None,
    );
    let names: Vec<_> = found
        .iter()
        .map(|one| {
            (
                one.name.as_str(),
                one.project_id.as_deref(),
                one.card_id.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [
            ("api-worker", Some("prj_1"), None),
            // A card's checkout belongs to the card's project.
            ("card-worker", Some("prj_1"), Some("card_7"))
        ]
    );
}

#[test]
fn a_dead_session_and_the_orchestrators_own_are_left_out() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sessions = dir.path().join("sessions");
    let orch = dir.path().join("orchestrator").join("claude");
    for path in [&sessions, &orch] {
        std::fs::create_dir_all(path).expect("mkdir");
    }
    listing(&sessions, 20, "gone", dir.path());
    listing(&sessions, 21, "me", &orch);

    let found = read(
        &sessions,
        |pid| pid != 20,
        &[project("orch", &orch, Some("claude"))],
        dir.path(),
        &HashMap::new(),
        |_| None,
    );
    assert!(
        found.is_empty(),
        "{:?}",
        found.iter().map(|one| &one.name).collect::<Vec<_>>()
    );
}

#[test]
fn only_devpits_card_checkouts_name_a_card() {
    let worktrees = Path::new("/home/me/.devpit/worktrees");
    assert_eq!(
        card_of(worktrees, &worktrees.join("prj_1/card_9/src")).as_deref(),
        Some("card_9")
    );
    assert_eq!(card_of(worktrees, &worktrees.join("prj_1/other")), None);
    assert_eq!(card_of(worktrees, Path::new("/elsewhere/card_9")), None);
}

#[test]
fn only_a_devpit_terminal_can_be_typed_into() {
    let none = HashMap::new();
    // An older CLI listed the client alone.
    assert_eq!(
        pane_target("devpit_prj_1__leaf_9", &none).as_deref(),
        Some("devpit_prj_1:leaf_9")
    );
    // As the CLI lists it now: the client, then tmux's window and pane ids.
    assert_eq!(
        pane_target(
            "devpit_prj_1__leaf_9:@26.%26",
            &windows(&[("%26", "leaf_9")])
        )
        .as_deref(),
        Some("devpit_prj_1:leaf_9")
    );
    assert_eq!(pane_target("devpit_prj_1__leaf_9:@2;rm", &none), None);
    assert_eq!(pane_target("main", &none), None);
    assert_eq!(pane_target("other_prj__leaf_9", &none), None);
    assert_eq!(pane_target("devpit_prj;rm__leaf_9", &none), None);
    assert_eq!(
        pane_target("devpit_prj_1__leaf_9:@2.%2", &windows(&[("%2", "rm;x")])),
        None
    );
}

/// After a restart, the CLI can record another pane's client session: every
/// client of a group shows every window, and tmux named an orphan one. The
/// pane id is the CLI's own and right, and it decides.
#[test]
fn a_session_is_placed_in_its_own_pane_whatever_client_the_cli_recorded() {
    let now = windows(&[("%2", "leaf_mine"), ("%5", "leaf_other")]);
    assert_eq!(
        pane_target("devpit_prj_1__leaf_other:@2.%2", &now).as_deref(),
        Some("devpit_prj_1:leaf_mine")
    );
    // A pane tmux no longer has is no terminal at all, never a neighbour's.
    assert_eq!(pane_target("devpit_prj_1__leaf_other:@9.%9", &now), None);
}

#[test]
fn a_session_in_a_devpit_terminal_says_the_question_it_is_stopped_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sessions = dir.path().join("sessions");
    std::fs::create_dir_all(&sessions).expect("mkdir");
    let body = json!({ "pid": 30, "name": "asking", "status": "idle", "kind": "interactive",
        "cwd": dir.path().display().to_string(), "tmux": "devpit_prj_1__leaf_1" });
    std::fs::write(sessions.join("30.json"), body.to_string()).expect("listing");

    let screen = "Proceed?\n❯ 1. Yes\n  2. No\n";
    let found = read(
        &sessions,
        |_| true,
        &[],
        dir.path(),
        &HashMap::new(),
        |target| (target == "devpit_prj_1:leaf_1").then(|| screen.to_owned()),
    );
    let waiting = found[0].waiting.as_ref().expect("waiting on a question");
    assert_eq!(waiting.question, "Proceed?");
    assert_eq!(waiting.options.len(), 2);
}

#[test]
fn a_devpit_terminal_is_opened_by_its_project_and_pane() {
    let target = pane_target(
        "devpit_prj_01AB__leaf_01CD:@3.%3",
        &windows(&[("%3", "leaf_01CD")]),
    )
    .expect("a target");
    assert_eq!(
        pane_of(&target),
        Some(devpit_rpc::LivePane {
            project_id: "prj_01AB".into(),
            pane_id: "leaf_01CD".into(),
        })
    );
}

/// A listing names a process by pid and start; a pid alone is reused after a
/// restart.
#[test]
fn a_process_start_is_read_past_a_command_with_spaces() {
    let stat = "12490 (claude (x) y) S 11194 12490 11194 34816 12490 4194560 1 2 3 4 5 6 7 8 20 0 12 0 5646 1 2";
    assert_eq!(start_of(stat), Some("5646"));
    assert_eq!(start_of("garbage"), None);
}

/// The kept project list is reused only while it names the projects there
/// are: one added since is a reason to read them again.
#[test]
fn a_project_added_since_the_list_was_kept_is_a_reason_to_read_it_again() {
    let there = vec!["prj_a".to_owned(), "prj_b".to_owned()];
    assert!(super::same_ids(["prj_b", "prj_a"].into_iter(), &there));
    assert!(!super::same_ids(["prj_a"].into_iter(), &there));
    assert!(!super::same_ids(["prj_a", "prj_c"].into_iter(), &there));
}
