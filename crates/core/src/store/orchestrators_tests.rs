use serde_json::json;

use super::*;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

/// The point of the move: what the orchestrator writes in its own folder no
/// longer decides what it is linked to.
#[test]
fn a_settings_file_edited_after_it_was_brought_in_is_not_read() {
    let (dir, store) = store();
    let folder = dir.path().join("orchestrator").join("work");
    let file = folder.join(ORCHESTRATOR_SETTINGS);
    std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
    std::fs::write(&file, r#"{"profile":"claude","projects":["prj_a"]}"#).expect("write");

    let first = store.orchestrator_settings(&folder).expect("read");
    assert_eq!(first["projects"], json!(["prj_a"]));

    std::fs::write(
        &file,
        r#"{"profile":"other","projects":["prj_a","prj_secret"]}"#,
    )
    .expect("edited by the orchestrator");
    let after = store.orchestrator_settings(&folder).expect("read again");
    assert_eq!(after["projects"], json!(["prj_a"]));
    assert_eq!(after["profile"], json!("claude"));
}

#[test]
fn one_setting_changes_and_the_others_stay() {
    let (dir, store) = store();
    let folder = dir.path().join("work");
    store
        .set_orchestrator_setting(&folder, "profile", json!("claude"))
        .expect("account");
    store
        .set_orchestrator_setting(&folder, "projects", json!(["prj_a"]))
        .expect("links");
    let all = store.orchestrator_settings(&folder).expect("read");
    assert_eq!(all["profile"], json!("claude"));
    assert_eq!(all["projects"], json!(["prj_a"]));
}
