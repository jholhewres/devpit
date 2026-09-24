use super::*;

fn project(id: &str, root: &Path, worktrees: &[&Path]) -> Project {
    serde_json::from_value(json!({
        "id": id,
        "name": id,
        "rootPath": root.display().to_string(),
        "group": null,
        "accent": "#000000",
        "worktrees": worktrees.iter().map(|tree| json!({
            "id": tree.display().to_string(),
            "branch": "main",
            // As git lists it: the last segment here, the whole path apart.
            "folder": tree.file_name().map(|name| name.to_string_lossy().into_owned()),
            "path": tree.display().to_string(),
            "ahead": 0,
            "behind": 0,
            "dirtyFiles": null,
            "origin": "devpit",
            "current": false,
        })).collect::<Vec<_>>(),
        "unreadable": null,
        "origin": null,
        "lastOpenedAt": null,
        "icon": null,
        "color": null,
    }))
    .expect("a project")
}

#[test]
fn the_project_is_the_one_whose_checkout_holds_the_agent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = dir.path().join("app");
    let tree = dir.path().join("worktrees").join("card-1");
    let deep = app.join("src").join("lib");
    for path in [&app, &tree, &deep] {
        std::fs::create_dir_all(path).expect("mkdir");
    }
    let projects = vec![
        project("app", &app, &[&tree]),
        project("other", &dir.path().join("x"), &[]),
    ];

    assert_eq!(
        project_at(&projects, &deep).map(|one| one.id.as_str()),
        Some("app")
    );
    // A card's worktree lives outside the root and is still the project's.
    assert_eq!(
        project_at(&projects, &tree).map(|one| one.id.as_str()),
        Some("app")
    );
    assert!(project_at(&projects, dir.path()).is_none());
}

/// A folder named like the root but beside it is not inside it.
#[test]
fn a_sibling_with_the_same_prefix_is_not_the_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = dir.path().join("app");
    let sibling = dir.path().join("app-old");
    std::fs::create_dir_all(&app).expect("mkdir");
    std::fs::create_dir_all(&sibling).expect("mkdir");
    assert!(project_at(&[project("app", &app, &[])], &sibling).is_none());
}

#[test]
fn a_question_it_cannot_read_or_answer_says_so() {
    assert!(answer(None, "not json").contains("\"error\""));
    let unknown = answer(None, r#"{"method":"card_move","cwd":"/"}"#);
    assert!(unknown.contains("does not answer `card_move`"), "{unknown}");
}

#[test]
fn it_says_what_it_answers() {
    let said: Value = serde_json::from_str(&answer(None, r#"{"method":"methods"}"#)).expect("json");
    assert_eq!(said["ok"], json!(METHODS));
}

#[test]
fn a_comment_is_signed_by_an_agent_devpit_knows_or_by_nobody_in_particular() {
    assert_eq!(signed("claude"), "claude");
    assert_eq!(signed("codex"), "codex");
    assert_eq!(signed("root"), "agent");
    assert_eq!(signed(""), "agent");
}

/// A lane that runs a step starts work when a card enters it: not an agent's
/// move to make.
#[test]
fn a_lane_with_a_step_is_behind_the_gate() {
    let lane = |step: Value| -> devpit_rpc::Column {
        serde_json::from_value(json!({
            "id": "col_1", "name": "Review", "position": 1, "step": step,
            "onPass": null, "autonomy": "manual"
        }))
        .expect("a column")
    };
    assert!(gate(&lane(Value::Null)).is_ok());
    let step = json!({ "id": "stp_1", "kind": "command", "name": "Tests", "config": "{}", "irreversible": false });
    let refused = gate(&lane(step)).expect_err("gated");
    assert!(refused.contains("`Review` runs a step"), "{refused}");
}

fn orchestrator(id: &str, root: &Path) -> Project {
    let mut one = project(id, root, &[]);
    one.orchestrator = Some("claude".to_owned());
    one
}

#[test]
fn only_an_orchestrator_reaches_another_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let projects = vec![
        project("prj_app", &dir.path().join("app"), &[]),
        project("prj_api", &dir.path().join("api"), &[]),
        orchestrator("prj_orch", &dir.path().join("orch")),
    ];
    let (app, orch) = (&projects[0], &projects[2]);

    assert_eq!(
        reached(&projects, orch, Some("prj_api")).map(|one| one.id.as_str()),
        Ok("prj_api")
    );
    assert_eq!(
        reached(&projects, orch, Some("PRJ_API")).map(|one| one.id.as_str()),
        Ok("prj_api")
    );
    assert_eq!(
        reached(&projects, app, None).map(|one| one.id.as_str()),
        Ok("prj_app")
    );
    assert!(
        reached(&projects, app, Some("prj_api")).is_err(),
        "a project agent reached another project"
    );
    assert!(
        reached(&projects, orch, Some("prj_orch")).is_err(),
        "an orchestrator was reachable as a project"
    );
}
