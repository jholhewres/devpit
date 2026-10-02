use serde_json::{json, Value};

use super::{diff, plan, put_ours, read, take_ours, write};

fn file(dir: &tempfile::TempDir, text: &str) -> std::path::PathBuf {
    let path = dir.path().join("settings.json");
    std::fs::write(&path, text).expect("write");
    path
}

#[test]
fn a_file_that_is_not_a_json_object_is_refused_never_taken_for_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    for broken in ["{ not json", "[1, 2]", "\"text\""] {
        let path = file(&dir, broken);
        assert!(read(&path).is_err(), "{broken} was read");
    }
    // Missing is empty; so is a blank file; a BOM is not a reason to refuse.
    assert!(read(&dir.path().join("none.json"))
        .expect("missing")
        .value
        .is_empty());
    let path = file(&dir, "\u{feff}{\"theme\":\"dark\"}");
    assert_eq!(read(&path).expect("bom").value["theme"], "dark");
}

#[test]
fn a_change_is_shown_backed_up_and_written_whole() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = file(&dir, "{\n  \"theme\": \"dark\"\n}\n");
    let loaded = read(&path).expect("read");
    let planned = plan(&loaded, |map| {
        put_ours(map, &["hooks"], "devpit", json!({ "on": true }))
    });
    assert!(planned.changes);
    assert!(planned.diff.contains("+  \"hooks\": {"), "{}", planned.diff);
    assert!(planned.diff.contains(" \"theme\": \"dark\""));

    let backup = write(&loaded, &planned, "20261002-101500")
        .expect("written")
        .expect("a backup");
    assert_eq!(
        std::fs::read_to_string(&backup).expect("backup"),
        "{\n  \"theme\": \"dark\"\n}\n"
    );
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("json");
    assert_eq!(written["theme"], "dark");
    assert_eq!(written["hooks"]["devpit"]["on"], true);
    assert!(
        !dir.path().join(".settings.json.devpit-writing").exists(),
        "no temporary left"
    );
}

/// Edited by hand between reading and writing: refused, untouched.
#[test]
fn a_file_changed_since_it_was_read_is_not_written() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = file(&dir, "{}");
    let loaded = read(&path).expect("read");
    let planned = plan(&loaded, |map| put_ours(map, &["hooks"], "devpit", json!(1)));
    std::fs::write(&path, "{\"mine\": true}").expect("edited meanwhile");
    assert!(write(&loaded, &planned, "x").is_err());
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "{\"mine\": true}"
    );
}

/// Installing and taking it out again leaves the person's settings as they
/// were, key for key.
#[test]
fn putting_ours_and_taking_it_out_gives_back_the_rest() {
    let mut map = json!({ "theme": "dark", "hooks": { "BeforeTool": [1] } })
        .as_object()
        .cloned()
        .expect("object");
    let before = map.clone();
    put_ours(&mut map, &["hooks"], "devpit", json!({ "x": 1 }));
    put_ours(&mut map, &["devpit-only", "deep"], "devpit", json!(2));
    take_ours(&mut map, &["hooks"], "devpit");
    take_ours(&mut map, &["devpit-only", "deep"], "devpit");
    assert_eq!(map, before);
}

/// A dotfiles link stays a link: the file it points at is what is written.
#[cfg(unix)]
#[test]
fn a_symlinked_file_is_written_through_its_link() {
    let dir = tempfile::tempdir().expect("tempdir");
    let real = dir.path().join("dotfiles-settings.json");
    std::fs::write(&real, "{}").expect("real");
    let link = dir.path().join("settings.json");
    std::os::unix::fs::symlink(&real, &link).expect("link");
    let loaded = read(&link).expect("read");
    let planned = plan(&loaded, |map| put_ours(map, &[], "devpit", json!(true)));
    write(&loaded, &planned, "t").expect("written");
    assert!(std::fs::symlink_metadata(&link)
        .expect("meta")
        .file_type()
        .is_symlink());
    assert!(std::fs::read_to_string(&real)
        .expect("real")
        .contains("devpit"));
}

#[test]
fn the_diff_shows_what_changed_with_its_context() {
    let shown = diff("a\nb\nc\n", "a\nB\nc\n", "f.json");
    assert!(shown.starts_with("--- f.json\n+++ f.json\n"));
    assert!(shown.contains("-b\n") && shown.contains("+B\n") && shown.contains(" a\n"));
    assert_eq!(diff("same\n", "same\n", "f"), "");
}
