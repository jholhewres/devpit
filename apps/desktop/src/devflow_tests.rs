use devpit_devflow::config::parse;
use devpit_devflow::Invalid;

use super::*;

fn valid(text: &str) -> State {
    State::Valid(text.to_owned(), Box::new(parse(text).expect("valid")))
}

#[test]
fn devflow_is_offered_only_where_the_file_and_the_click_both_say_so() {
    let on = valid("version: 1\nenabled: true\nstages:\n  context: { enabled: false }\n");
    assert_eq!(
        offered(true, &on, true),
        Offered::Active(vec![
            "briefing",
            "plan",
            "execution",
            "review",
            "completion"
        ])
    );
    // The file alone is a hand edit or a clone: the person has not clicked.
    assert_eq!(offered(true, &on, false), Offered::Setup);
    assert_eq!(
        offered(true, &valid("version: 1\nenabled: false\n"), true),
        Offered::Setup
    );
    assert_eq!(offered(true, &State::Missing, false), Offered::Setup);
    let invalid = State::Invalid(None, parse("version: 9\n").expect_err("invalid"));
    assert_eq!(offered(true, &invalid, true), Offered::Setup);
    // Off in Settings: what devpit offered before Devflow, file or not.
    assert_eq!(offered(false, &on, true), Offered::Off);
    assert_eq!(offered(false, &State::Missing, false), Offered::Off);
}

#[test]
fn the_state_is_said_in_the_skills_words() {
    let invalid = State::Invalid(
        None,
        Invalid {
            line: Some(2),
            field: "enabled".into(),
            message: "expected true or false".into(),
        },
    );
    assert_eq!(state_word(&State::Missing, false), "no file");
    assert_eq!(state_word(&invalid, false), "invalid");
    assert_eq!(state_word(&valid("version: 1\n"), false), "disabled");
    assert_eq!(
        state_word(&valid("version: 1\nenabled: true\n"), true),
        "active"
    );
}

fn project(id: &str, root: &Path) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": null,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

#[test]
fn a_setup_writes_only_when_applied_and_leaves_switching_on_to_the_click() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("Makefile"), "test:\n\ttrue\n").expect("makefile");
    let here = project("prj_devflow_setup", dir.path());
    let ask = |method: &str, params: Value| {
        respond(None, method, &here, dir.path(), &params).expect(method)
    };

    let status = ask("devflow_status", json!({}));
    assert_eq!(status["state"], "no file");
    assert_eq!(status["recommended_checks"][0]["run"], "make test");
    assert_eq!(
        ask("devflow_offer", json!({}))["offer"],
        "setup",
        "the setup tools are offered with no file"
    );

    let preview = ask("devflow_setup", json!({ "preset": "recommended" }));
    assert_eq!(preview["applied"], false);
    assert!(!dir.path().join(".devpit").exists(), "a preview wrote");

    let applied = ask(
        "devflow_setup",
        json!({ "preset": "recommended", "apply": true }),
    );
    assert_eq!(applied["applied"], true);
    for file in [
        "devflow.yaml",
        ".gitignore",
        "scripts/check-unit.sh",
        "reviewers/devflow-security.md",
        "devflow-findings.schema.json",
    ] {
        assert!(
            dir.path().join(".devpit/devflow").join(file).is_file(),
            "{file}"
        );
    }
    assert_eq!(applied["waiting_for_click"], json!(["enable"]));
    assert_eq!(ask("devflow_status", json!({}))["state"], "disabled");

    // An agent cannot switch it on: the file stays as it was.
    let text = || std::fs::read_to_string(dir.path().join(FILE)).expect("yaml");
    let before = text();
    let refused = ask(
        "devflow_set",
        json!({ "path": "enabled", "value": true, "apply": true }),
    );
    assert_eq!(refused["applied"], false);
    assert_eq!(text(), before);
    assert_eq!(ask("devflow_offer", json!({}))["offer"], "setup");

    let changed = ask(
        "devflow_set",
        json!({ "path": "review.block_on", "value": ["critical"], "apply": true }),
    );
    assert_eq!(changed["applied"], true);
    assert!(text().contains("block_on: [critical]"), "{}", text());
}

/// The server lists from this answer: an orchestrator gets its own tools and
/// no flow to set up in its notes; a project, neither of an orchestrator's.
#[test]
fn the_offer_says_who_asks_and_an_orchestrator_is_offered_no_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut here = project("prj_offer_seat", dir.path());
    let offer = |here: &Project| {
        respond(None, "devflow_offer", here, dir.path(), &json!({})).expect("offer")
    };
    let plain = offer(&here);
    assert_eq!(plain["offer"], "setup");
    assert_eq!(plain["orchestrator"], false);
    assert!(plain["decisions"].is_boolean());
    here.orchestrator = Some("claude".to_owned());
    let orchestrating = offer(&here);
    assert_eq!(orchestrating["offer"], "off");
    assert_eq!(orchestrating["orchestrator"], true);
}

#[test]
fn every_devflow_method_is_answered_by_the_agent_api() {
    for method in METHODS {
        assert!(crate::agent_api::METHODS.contains(&method), "{method}");
    }
}

/// A team's layer as a bare repository, at tag v1, locking its reviewer.
fn team_layer(base: &Path) -> String {
    let run = |dir: &Path, args: &[&str]| {
        let ran = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.email=team@example.invalid",
                "-c",
                "user.name=Team",
            ])
            .args(["-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false"])
            .args(args)
            .output()
            .expect("git");
        assert!(ran.status.success(), "git {args:?}");
    };
    let work = base.join("work");
    std::fs::create_dir_all(&work).expect("work");
    run(&work, &["init", "-q"]);
    std::fs::write(
        work.join("devflow.yaml"),
        "version: 2\nreview:\n  reviewers:\n    - agent: claude\n      persona: devflow-security\nlocked: [review.reviewers.devflow-security]\n",
    )
    .expect("layer");
    run(&work, &["add", "-A"]);
    run(&work, &["commit", "-qm", "one"]);
    run(&work, &["tag", "v1"]);
    let bare = base.join("acme").join("flow.git");
    run(base, &["clone", "-q", "--bare", "work", "acme/flow.git"]);
    bare.display().to_string()
}

#[test]
fn a_change_that_weakens_what_the_team_locked_is_refused_before_it_is_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let url = team_layer(dir.path());
    let root = dir.path().join("project");
    std::fs::create_dir_all(root.join(".devpit/devflow")).expect("folder");
    let text = format!("version: 2\nextends:\n  - git: {url}\n    ref: v1\n");
    std::fs::write(root.join(FILE), &text).expect("yaml");
    let layer = devpit_devflow::config::Extend {
        git: url,
        reference: "v1".into(),
    };
    let shown = devpit_devflow::pinning::fetched(&root, &layer, &home()).expect("fetched");
    devpit_devflow::pinning::approve(&root, &layer, &shown.to_commit, &home()).expect("approved");
    let here = project("prj_devflow_locked", &root);

    let refused = respond(
        None,
        "devflow_set",
        &here,
        &root,
        &json!({ "path": "review.reviewers", "value": [], "apply": true }),
    )
    .expect_err("the team locked it");
    assert!(refused.contains("locked by acme"), "{refused}");
    assert_eq!(
        std::fs::read_to_string(root.join(FILE)).expect("yaml"),
        text
    );
    let State::Valid(_, config) = read(&root) else {
        panic!("the layered file reads")
    };
    assert_eq!(config.review.reviewers[0].persona, "devflow-security");
}

#[test]
fn a_field_this_devpit_does_not_read_is_said_in_the_status_note() {
    let unread = devpit_devflow::config::unread("version: 2\nreview:\n  scale: risk\n");
    let note = unread_note(&unread).expect("a note");
    assert!(note.contains("review.scale"), "{note}");
    assert!(note.contains("line 3"), "{note}");
    assert_eq!(unread_note(&[]), None);
}
