use std::path::Path;

use devpit_rpc::Project;
use serde_json::json;

use super::{proposal_of, Asked};

fn project(id: &str, root: &Path) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": null,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

fn asked<'a>(path: Option<&'a str>, project: Option<&'a str>) -> Asked<'a> {
    Asked {
        path,
        project,
        name: None,
        group: None,
        link: None,
    }
}

#[test]
fn a_new_folder_is_proposed_by_its_name_and_linked() {
    let home = tempfile::tempdir().expect("home");
    std::fs::create_dir_all(home.path().join("Workspace/asc-api")).expect("folder");
    let proposal = proposal_of(
        &[],
        &[],
        Some(home.path()),
        &Asked {
            group: Some(" ASC "),
            ..asked(Some("~/Workspace/asc-api"), None)
        },
    )
    .expect("proposal");
    assert_eq!(proposal.project_id, None);
    assert_eq!(proposal.name, "asc-api");
    assert_eq!(proposal.group.as_deref(), Some("ASC"));
    assert_eq!(proposal.link, Some(true));
    assert!(
        proposal.path.ends_with("Workspace/asc-api"),
        "{}",
        proposal.path
    );
}

#[test]
fn a_known_project_is_proposed_only_for_what_would_change() {
    let dir = tempfile::tempdir().expect("dir");
    let api = project("prj_api", dir.path());
    let path = dir.path().display().to_string();

    // Unlinked, its folder proposes linking it.
    let proposal = proposal_of(
        std::slice::from_ref(&api),
        &[],
        None,
        &asked(Some(&path), None),
    )
    .expect("proposal");
    assert_eq!(
        (proposal.project_id.as_deref(), proposal.link),
        (Some("prj_api"), Some(true))
    );

    // Linked already, with nothing else asked, there is nothing to propose.
    let linked = ["prj_api".to_owned()];
    assert!(proposal_of(
        std::slice::from_ref(&api),
        &linked,
        None,
        &asked(None, Some("prj_api"))
    )
    .is_err());

    // A group, by name, while the link stays as it is.
    let regroup = proposal_of(
        std::slice::from_ref(&api),
        &linked,
        None,
        &Asked {
            group: Some("ASC"),
            ..asked(None, Some("prj_api"))
        },
    )
    .expect("proposal");
    assert_eq!(
        (regroup.link, regroup.group.as_deref()),
        (None, Some("ASC"))
    );

    // And unlinking, said outright.
    let unlink = proposal_of(
        std::slice::from_ref(&api),
        &linked,
        None,
        &Asked {
            link: Some(false),
            ..asked(None, Some("prj_api"))
        },
    )
    .expect("proposal");
    assert_eq!(unlink.link, Some(false));
}

#[test]
fn a_folder_that_is_not_there_or_not_whole_is_refused() {
    assert!(proposal_of(&[], &[], None, &asked(Some("relative/x"), None)).is_err());
    assert!(proposal_of(&[], &[], None, &asked(Some("/nowhere/at/all"), None)).is_err());
    assert!(proposal_of(&[], &[], None, &asked(None, None)).is_err());
    assert!(proposal_of(&[], &[], None, &asked(None, Some("ghost"))).is_err());
}
