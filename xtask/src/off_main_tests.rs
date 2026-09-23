//! A command that runs on the window's thread.

use super::*;

const FILE: &str = r#"
#[tauri::command]
#[specta::specta]
pub fn waits(project_id: String) -> Result<(), RpcError> {
    Ok(())
}

/// Documented, and async.
#[tauri::command]
#[specta::specta]
pub async fn does_not(project_id: String) -> Result<(), RpcError> {
    Ok(())
}

pub fn not_a_command() {}

#[tauri::command]
// a comment between them
pub fn also_waits() {}
"#;

#[test]
fn a_plain_fn_command_is_found_and_an_async_one_is_not() {
    let found = sync_commands(FILE);
    let names: Vec<&str> = found.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["waits", "also_waits"]);
    assert_eq!(found[0].1, 4);
}

#[test]
fn a_listed_command_is_allowed_and_an_unlisted_one_fails() {
    let dir = tempfile::tempdir().expect("tempdir");
    let src = dir.path().join(SOURCES);
    std::fs::create_dir_all(&src).expect("tree");
    std::fs::write(src.join("one.rs"), FILE).expect("file");
    // A test file is never a command's home.
    std::fs::write(src.join("one_tests.rs"), FILE).expect("file");

    let found = held_in(dir.path(), "waits answers from memory\n");
    assert_eq!(
        found.len(),
        1,
        "{:?}",
        found.iter().map(ToString::to_string).collect::<Vec<_>>()
    );
    assert!(found[0].what.contains("also_waits"));
    assert!(found[0].file.ends_with("one.rs"));
}

/// The shapes a line-by-line reading used to let through.
#[test]
fn every_way_of_writing_a_sync_command_is_found() {
    let text = r#"
#[tauri::command(
    rename_all = "snake_case",
)]
pub(super) fn wrapped() {}

#[tauri::command]
/* a note
   over lines */
pub(in crate::m) fn commented() {}

#[tauri::command] pub fn one_line() {}

#[tauri::command]
#[specta::specta]
pub(crate) async fn fine() {}
"#;
    let names: Vec<String> = sync_commands(text)
        .into_iter()
        .map(|(name, _)| name)
        .collect();
    assert_eq!(names, ["wrapped", "commented", "one_line"]);
}
