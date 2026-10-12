use super::*;
use crate::store::settings::key;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

#[test]
fn a_preference_says_when_it_was_set() {
    let (_dir, store) = store();
    assert_eq!(store.preference_stamped(key::THEME).expect("read"), None);
    store.set_preference(key::THEME, "light").expect("write");
    let (value, at) = store
        .preference_stamped(key::THEME)
        .expect("read")
        .expect("set");
    assert_eq!(value, "light");
    assert!(at > 1_700_000_000, "seconds since the epoch: {at}");
}

#[test]
fn two_clones_of_one_repository_share_an_origin_in_a_steady_order() {
    let (dir, store) = store();
    let first = store
        .add_project(
            &dir.path().join("one"),
            Some("git@github.com:someone/thing.git"),
        )
        .expect("add");
    let second = store
        .add_project(
            &dir.path().join("two"),
            Some("https://github.com/someone/thing"),
        )
        .expect("add");
    let alone = store
        .add_project(&dir.path().join("alone"), None)
        .expect("add");
    let listed = store.synced_projects().expect("list");
    let origin = |id: &str| {
        listed
            .iter()
            .find(|one| one.id == id)
            .map(|one| one.origin_hash.clone())
            .expect("listed")
    };
    assert!(origin(&first).is_some());
    assert_eq!(origin(&first), origin(&second));
    assert_eq!(origin(&alone), None);
    let order: Vec<(i64, &str)> = listed
        .iter()
        .map(|one| (one.created_at, one.id.as_str()))
        .collect();
    let mut sorted = order.clone();
    sorted.sort();
    assert_eq!(order, sorted, "by age, then by id, so every read agrees");
}

#[test]
fn a_plugin_row_is_set_as_another_machine_left_it() {
    let (dir, store) = store();
    let project = store.add_project(dir.path(), None).expect("add");
    store
        .set_plugin_row(&project, "notes", true, true)
        .expect("set");
    assert_eq!(store.enabled_plugins(&project).expect("on"), vec!["notes"]);
    store
        .set_plugin_row(&project, "notes", false, true)
        .expect("removed");
    let rows = store.plugin_rows(&project).expect("rows");
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].installed);
    assert!(!rows[0].enabled, "a plugin removed is not on");
    assert!(store.enabled_plugins(&project).expect("on").is_empty());
}
