use std::path::Path;

use devpit_rpc::Project;
use serde_json::json;

use super::{linked, reaches, set};

fn project(id: &str, root: &Path, orchestrator: Option<&str>) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": orchestrator,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

#[test]
fn linking_keeps_the_account_and_the_account_keeps_the_links() {
    let dir = tempfile::tempdir().expect("tempdir");
    set(dir.path(), "profile", json!("claude")).expect("account");
    set(dir.path(), "projects", json!(["prj_a", "prj_b"])).expect("links");
    assert_eq!(linked(dir.path()), ["prj_a", "prj_b"]);
    set(dir.path(), "profile", json!("claudin")).expect("account again");
    assert_eq!(linked(dir.path()), ["prj_a", "prj_b"]);
    assert_eq!(
        devpit_core::home::orchestrator_profile(dir.path()).as_deref(),
        Some("claudin")
    );
}

#[test]
fn an_orchestrator_reaches_only_what_it_is_linked_to() {
    let dir = tempfile::tempdir().expect("tempdir");
    let orch = project("prj_orch", dir.path(), Some("claude"));
    let linked_one = project("prj_a", Path::new("/work/a"), None);
    let other = project("prj_b", Path::new("/work/b"), None);
    set(dir.path(), "projects", json!(["prj_a"])).expect("links");

    assert!(reaches(&orch, &linked_one).is_ok());
    assert!(reaches(&orch, &orch).is_ok());
    let refused = reaches(&orch, &other).expect_err("not linked");
    assert!(refused.contains("not linked"), "{refused}");
    // A project is not an orchestrator: this rule is not its.
    assert!(reaches(&linked_one, &linked_one).is_ok());
}

#[test]
fn nothing_is_linked_until_the_person_links_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(linked(dir.path()).is_empty());
}
