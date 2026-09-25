use std::path::Path;

use devpit_rpc::Project;
use serde_json::json;

use super::{card_of, pane_target, read};

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
            ("card-worker", None, Some("card_7"))
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
    assert_eq!(
        pane_target("devpit_prj_1__leaf_9").as_deref(),
        Some("devpit_prj_1__leaf_9:leaf_9")
    );
    assert_eq!(pane_target("main"), None);
    assert_eq!(pane_target("other_prj__leaf_9"), None);
    assert_eq!(pane_target("devpit_prj;rm__leaf_9"), None);
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
        |target| (target == "devpit_prj_1__leaf_1:leaf_1").then(|| screen.to_owned()),
    );
    let waiting = found[0].waiting.as_ref().expect("waiting on a question");
    assert_eq!(waiting.question, "Proceed?");
    assert_eq!(waiting.options.len(), 2);
}
