use std::process::Command;

use devpit_devflow::Ask;
use serde_json::json;

use super::*;

fn project_at(id: &str, root: &Path) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": null,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

fn git(at: &Path, args: &[&str]) {
    let done = Command::new("git")
        .args(args)
        .current_dir(at)
        .output()
        .expect("git");
    assert!(done.status.success(), "git {args:?}: {done:?}");
}

/// A git checkout with one commit and a change on top, Devflow on with one
/// reviewer, and a card of its plan whose checkout it is.
fn reviewed_project(dir: &Path) -> (Store, Project, String) {
    git(dir, &["init", "-q", "-b", "main"]);
    git(dir, &["config", "user.email", "t@example.com"]);
    git(dir, &["config", "user.name", "Test"]);
    std::fs::write(dir.join("app.py"), "print('hi')\n").expect("write");
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-q", "-m", "start"]);
    std::fs::write(dir.join("app.py"), "import os\nos.system(link)\n").expect("change");

    let store = crate::projects::store().expect("store");
    let id = store.add_project(dir, None).expect("project");
    store.ensure_board(&id).expect("board");
    std::fs::create_dir_all(dir.join(".devpit/devflow")).expect("folder");
    std::fs::write(
        dir.join(".devpit/devflow/devflow.yaml"),
        "version: 1\nenabled: true\nreview:\n  reviewers:\n    - agent: claude\n      persona: devflow-security\n",
    )
    .expect("yaml");
    crate::devflow_asks::remember(&store, &id, Ask::Enable, true).expect("clicked");
    let first = store.columns(&id).expect("columns")[0].id.clone();
    let card = store
        .create_card(
            &id,
            &first,
            "[devflow] Open links",
            "## Acceptance criteria\n- [ ] opens the first link\n",
        )
        .expect("card");
    let here = dir.display().to_string();
    store
        .set_card_front(&card, Some(&here), Some("HEAD"))
        .expect("its checkout");
    (store, project_at(&id, dir), card)
}

/// A script written by a child process: a file this process held open for
/// writing is inherited by whatever another test forks meanwhile, and
/// running it then fails with "text file busy".
#[cfg(unix)]
fn written_apart(path: &Path, text: &str) {
    let done = std::process::Command::new("sh")
        .args([
            "-c",
            "printf '%s' \"$1\" > \"$2\" && chmod 755 \"$2\"",
            "sh",
            text,
        ])
        .arg(path)
        .status()
        .expect("sh");
    assert!(done.success(), "{done}");
}

/// A fake Claude Code: one critical finding on the changed line.
#[cfg(unix)]
fn fake_claude(dir: &Path) -> String {
    let answer = json!({
        "version": 1,
        "reviewers": [{ "agent": "claude", "persona": "devflow-security" }],
        "verdict": "request_changes",
        "findings": [{
            "file": "app.py", "line": 2, "severity": "critical", "axis": "security",
            "title": "Shell injection", "description": "The link reaches a shell.",
            "evidence": "app.py:2 `os.system(link)`", "poc": "a title with `; rm -rf ~`",
            "agent": "claude", "persona": "devflow-security",
        }],
        "done_well": ["small change"],
        "verification": { "tests": "none ran", "evidence": "no checks" },
    });
    let wrapped = json!({ "type": "result", "is_error": false, "result": answer.to_string(), "total_cost_usd": 0.02 });
    let path = dir.join("claude-fake");
    written_apart(
        &path,
        &format!("#!/bin/sh\ncat > /dev/null\ncat <<'EOF'\n{wrapped}\nEOF\n"),
    );
    path.display().to_string()
}

fn config_of(root: &Path) -> Config {
    match devpit_devflow::folder::read(root) {
        devpit_devflow::State::Valid(_, config) => *config,
        other => panic!("{other:?}"),
    }
}

#[cfg(unix)]
#[test]
fn one_reviewer_on_a_fake_cli_reviews_the_card_and_its_critical_finding_holds_the_gate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let scripts = tempfile::tempdir().expect("scripts");
    let root = dir.path().canonicalize().expect("root");
    let (store, project, card) = reviewed_project(&root);
    let claude = fake_claude(scripts.path());
    let reach = move |agent: Agent| match agent {
        Agent::Claude => Ok(claude.clone()),
        _ => Err("not installed".to_owned()),
    };
    let said = respond_with(
        None,
        &project,
        &config_of(&root),
        &root,
        &json!({ "card": card }),
        &reach,
    )
    .expect("reviewed");
    assert_eq!(said["verdict"], "request_changes", "{said}");
    assert_eq!(said["reviewers"][0]["cost_usd"], 0.02);
    assert_eq!(
        said["pending"],
        json!(["devflow-spec", "devflow-standards"])
    );
    let packet = std::fs::read_to_string(root.join(said["packet"].as_str().expect("packet")))
        .expect("the packet");
    assert!(packet.contains("+os.system(link)"), "{packet}");

    let comment = store
        .comments(&card)
        .expect("comments")
        .last()
        .map(|comment| comment.body.clone())
        .unwrap_or_default();
    assert!(
        comment.starts_with("[devflow] review: 1 findings (1 critical, 0 high)"),
        "{comment}"
    );
    assert!(
        comment.contains("- claude devflow-security: 1 finding, "),
        "{comment}"
    );

    let gate = crate::devflow::respond(
        None,
        "devflow_gate",
        &project,
        &root,
        &json!({ "card": card }),
    )
    .expect("the gate");
    let missing = gate["missing"].to_string();
    assert!(
        missing.contains("finding `app.py:2` Shell injection (critical) is in block_on"),
        "{missing}"
    );
    assert!(
        missing.contains("has not covered devflow-spec, devflow-standards"),
        "{missing}"
    );

    // The card's diff draws the finding; called a false positive there, it
    // holds the gate no more.
    let heard = crate::checkpoint_findings::of_card(&store, &card).expect("findings");
    let review = &heard.reviews[0];
    assert!(review.run_id.starts_with("devflow:"), "{}", review.run_id);
    assert_eq!(review.found.findings[0].line, Some(2));
    assert_eq!(review.found.findings[0].severity, Severity::Blocking);
    mark(&store, &review.run_id, 0, true).expect("a false positive");
    let heard = crate::checkpoint_findings::of_card(&store, &card).expect("findings");
    assert_eq!(heard.reviews[0].dismissed, vec![0]);
    let gate = crate::devflow::respond(
        None,
        "devflow_gate",
        &project,
        &root,
        &json!({ "card": card }),
    )
    .expect("the gate");
    assert!(
        !gate["missing"].to_string().contains("Shell injection"),
        "{gate}"
    );
}

#[test]
fn a_devflow_review_is_named_apart_from_a_runs() {
    assert_eq!(
        devflow_run("devflow:card_1/20261011T143005Z"),
        Some(("card_1", "20261011T143005Z"))
    );
    assert_eq!(devflow_run("run_123"), None);
    assert_eq!(devflow_run("devflow:../x/y"), None);
}

#[test]
fn a_card_started_in_a_devflow_checkout_keeps_where_it_began() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let (_store, project, card) = reviewed_project(root);
    let head = devpit_git::head_of(root).expect("head");
    began(&project, root, &card);
    assert_eq!(
        devpit_devflow::started::start_of(root, &card),
        Some(head.clone())
    );

    // A second start, after more commits, does not move where it began.
    git(root, &["commit", "-q", "-am", "more"]);
    began(&project, root, &card);
    assert_eq!(devpit_devflow::started::start_of(root, &card), Some(head));
}

#[test]
fn a_checkout_without_devflow_keeps_no_start() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["config", "user.email", "t@example.com"]);
    git(root, &["config", "user.name", "Test"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", "start"]);
    began(&project_at("p", root), root, "card_01ABC");
    assert!(!root.join(".devpit").exists());
}
