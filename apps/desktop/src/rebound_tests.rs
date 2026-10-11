use super::*;

fn open(dir: &std::path::Path) -> devpit_core::Store {
    devpit_core::Store::open(&dir.join("state.db")).expect("open")
}

fn keys(pairs: &[(&str, &[&str])]) -> Rebound {
    Rebound {
        keys: pairs
            .iter()
            .map(|(id, keys)| (id.to_string(), keys.iter().map(|k| k.to_string()).collect()))
            .collect(),
    }
}

#[test]
fn a_fresh_install_has_every_default() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(read(&open(dir.path())), Rebound::default());
}

#[test]
fn a_recorded_key_survives_closing_the_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    let wanted = keys(&[("palette", &["Mod+J"])]);
    rebound_set_now(&open(dir.path()), wanted.clone()).expect("write");
    assert_eq!(read(&open(dir.path())), wanted);
}

#[test]
fn a_shortcut_with_no_key_is_not_kept() {
    // It would answer to nothing, which is not what a reset means.
    let dir = tempfile::tempdir().expect("tempdir");
    let kept = rebound_set_now(
        &open(dir.path()),
        keys(&[("palette", &[]), ("project", &["", "Mod+O"])]),
    )
    .expect("write");
    assert_eq!(kept, keys(&[("project", &["Mod+O"])]));
}

#[test]
fn a_value_nobody_can_parse_reads_as_every_default() {
    // One bad row in a settings table must not take the keyboard with it.
    let dir = tempfile::tempdir().expect("tempdir");
    let store = open(dir.path());
    store
        .set_preference(preference::SHORTCUT_KEYS, "{ not json")
        .expect("write");
    assert_eq!(read(&store), Rebound::default());
}
