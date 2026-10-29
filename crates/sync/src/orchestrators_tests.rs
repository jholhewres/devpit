use super::*;

#[test]
fn the_same_folder_name_mints_the_same_id_on_every_machine() {
    assert_eq!(minted("work"), minted("work"));
    assert_ne!(minted("work"), minted("home"));
    assert!(plain_id(&minted("work")));
}

#[test]
fn a_value_from_elsewhere_is_held_to_its_rules() {
    let hash = "0123456789abcdef0123456789abcdef";
    assert!(fits(&format!("{LINK}{hash}"), Some("on")));
    assert!(!fits(&format!("{LINK}{hash}"), Some("maybe")));
    assert!(!fits(&format!("{LINK}prj_01ABC"), Some("on")));
    assert!(!fits(&format!("{LINK}../x"), Some("on")));
    assert!(
        !fits("profile", Some("claude-work")),
        "the account stays here"
    );
    assert!(!fits(NAME, Some("  ")));
    assert!(!fits("folder", Some("/home/someone")));
}

#[test]
fn an_orchestrator_is_its_folder_one_or_two_deep_and_nothing_under_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let add = |path: &str| {
        let folder = dir.path().join(path);
        std::fs::create_dir_all(&folder).expect("folder");
        store.add_project(&folder, None).expect("add")
    };
    add("orchestrator/work");
    add("orchestrator/alice/work");
    add("orchestrator/work/repos/x");
    add("elsewhere/work");
    let space = Orchestrators {
        store: &store,
        root: dir.path(),
    };

    let here = space.here().expect("here");

    assert_eq!(here.len(), 2, "{:?}", here.keys());
    assert!(here.contains_key(&identity(&minted("work"))));
    assert!(here.contains_key(&identity(&minted("alice/work"))));
}

#[test]
fn an_orchestrator_made_later_never_takes_an_id_another_kept() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let add = |path: &str| {
        let folder = dir.path().join(path);
        std::fs::create_dir_all(&folder).expect("folder");
        store.add_project(&folder, None).expect("add")
    };
    let space = Orchestrators {
        store: &store,
        root: dir.path(),
    };
    add("orchestrator/acct/work");
    assert_eq!(space.here().expect("here").len(), 1);

    add("orchestrator/work");
    let here = space.here().expect("here");

    assert_eq!(here.len(), 2, "{:?}", here.keys());
}
