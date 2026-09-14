use super::*;

fn open(dir: &std::path::Path) -> devpit_core::Store {
    devpit_core::Store::open(&dir.join("state.db")).expect("open")
}

#[test]
fn a_fresh_install_has_no_default_and_hides_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let choice = read(&open(dir.path()));
    assert_eq!(choice.default_id, "");
    assert_eq!(choice.disabled, Vec::<String>::new());
}

#[test]
fn a_default_survives_closing_the_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open(dir.path());
    store
        .set_preference(preference::AGENT_DEFAULT, "claude")
        .expect("write");
    drop(store);
    assert_eq!(read(&open(dir.path())).default_id, "claude");
}

#[test]
fn a_disabled_list_nobody_can_parse_reads_as_empty() {
    // One bad row in a settings table must not take the menu with it.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open(dir.path());
    store
        .set_preference(preference::AGENTS_DISABLED, "{ not json")
        .expect("write");
    assert_eq!(read(&store).disabled, Vec::<String>::new());
}

#[test]
fn a_default_that_is_switched_off_stops_being_the_default() {
    // A menu whose default is not in it is a menu that opens nothing and
    // explains nothing. Clearing is a state the screen can draw.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open(dir.path());
    store
        .set_preference(preference::AGENT_DEFAULT, "codex")
        .expect("write");

    let mut choice = read(&store);
    choice.disabled.push("codex".to_owned());
    store
        .set_preference(
            preference::AGENTS_DISABLED,
            &serde_json::to_string(&choice.disabled).expect("encode"),
        )
        .expect("write");
    store
        .set_preference(preference::AGENT_DEFAULT, "")
        .expect("clear");

    let after = read(&store);
    assert_eq!(after.default_id, "");
    assert_eq!(after.disabled, ["codex"]);
}

#[test]
fn the_hooks_are_on_until_somebody_says_otherwise() {
    // The sidebar saying whether an agent is waiting for you is what the flag
    // is for, and a fresh install should have it.
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(read(&open(dir.path())).hooks);
}

#[test]
fn switching_the_hooks_off_survives_closing_the_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open(dir.path());
    store
        .set_preference_flag(preference::AGENT_HOOKS, false)
        .expect("write");
    drop(store);
    assert!(!read(&open(dir.path())).hooks);
}
