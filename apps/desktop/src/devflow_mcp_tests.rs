use std::path::Path;

use serde_json::{json, Value};

use super::*;

const CMD: &str = "/h/.local/bin/devflow";

fn json_of(text: &str) -> Value {
    serde_json::from_str(text).expect("json")
}

#[test]
fn each_tool_keeps_its_own_file() {
    let home = Path::new("/h");
    let none = |_: &str| None;
    assert_eq!(
        Tool::Codex.file(home, none),
        Path::new("/h/.codex/config.toml")
    );
    assert_eq!(
        Tool::Codex.file(home, |key| (key == "CODEX_HOME").then(|| "/c".to_owned())),
        Path::new("/c/config.toml")
    );
    assert_eq!(
        Tool::Cursor.file(home, none),
        Path::new("/h/.cursor/mcp.json")
    );
    assert_eq!(
        Tool::Opencode.file(home, none),
        Path::new("/h/.config/opencode/opencode.json")
    );
    assert_eq!(
        Tool::Opencode.file(home, |key| (key == "XDG_CONFIG_HOME")
            .then(|| "/x".to_owned())),
        Path::new("/x/opencode/opencode.json")
    );
    assert_eq!(
        Tool::Gemini.file(home, none),
        Path::new("/h/.gemini/settings.json")
    );
}

#[test]
fn cursor_and_gemini_get_it_under_mcp_servers_beside_theirs() {
    for tool in [Tool::Cursor, Tool::Gemini] {
        let theirs = r#"{"mcpServers":{"other":{"command":"x"}},"theme":"dark"}"#;
        assert!(!added_in(tool, theirs));
        let added = with(tool, Some(theirs), CMD).expect("added");
        assert!(added_in(tool, &added), "{added}");
        let value = json_of(&added);
        assert_eq!(
            value["mcpServers"]["devflow"],
            json!({ "command": CMD, "args": ["mcp"] })
        );
        assert_eq!(value["mcpServers"]["other"]["command"], "x");
        assert_eq!(value["theme"], "dark");
        let removed = without(tool, &added).expect("removed");
        assert!(!added_in(tool, &removed));
        assert_eq!(json_of(&removed)["mcpServers"]["other"]["command"], "x");
        assert!(added_in(tool, &with(tool, None, CMD).expect("a new file")));
    }
}

#[test]
fn opencode_gets_it_as_a_local_server() {
    let added = with(Tool::Opencode, Some(r#"{"model":"m"}"#), CMD).expect("added");
    let value = json_of(&added);
    assert_eq!(
        value["mcp"]["devflow"],
        json!({ "type": "local", "command": [CMD, "mcp"], "enabled": true })
    );
    assert_eq!(value["model"], "m");
    assert!(added_in(Tool::Opencode, &added));
    assert!(!added_in(
        Tool::Opencode,
        &without(Tool::Opencode, &added).expect("removed")
    ));
}

#[test]
fn codex_gets_its_own_table_and_loses_only_that_one() {
    let theirs = "model = \"o3\"\n\n[mcp_servers.other]\ncommand = \"x\"\n";
    let added = with(Tool::Codex, Some(theirs), r"C:\bin\devflow").expect("added");
    assert!(added.starts_with(theirs), "{added}");
    assert!(
        added.contains(
            "[mcp_servers.devflow]\ncommand = \"C:\\\\bin\\\\devflow\"\nargs = [\"mcp\"]\n"
        ),
        "{added}"
    );
    assert!(added_in(Tool::Codex, &added));
    let with_env =
        format!("{added}\n[mcp_servers.devflow.env]\nA = \"1\"\n\n[profiles.x]\nmodel = \"m\"\n");
    let removed = without(Tool::Codex, &with_env).expect("removed");
    assert!(!added_in(Tool::Codex, &removed), "{removed}");
    assert!(!removed.contains("A = \"1\""), "{removed}");
    assert!(
        removed.contains("[mcp_servers.other]\ncommand = \"x\""),
        "{removed}"
    );
    assert!(removed.contains("[profiles.x]\nmodel = \"m\""), "{removed}");
    let again = with(Tool::Codex, Some(&added), CMD).expect("again");
    assert_eq!(again.matches("[mcp_servers.devflow]").count(), 1, "{again}");
}

#[test]
fn a_file_that_is_not_json_is_left_alone() {
    assert!(with(Tool::Cursor, Some("{ not json"), CMD).is_err());
    assert!(without(Tool::Gemini, "[1, 2]").is_err());
}

#[test]
fn devflow_is_added_to_a_tool_and_removed_with_its_state_read_back() {
    let home = tempfile::tempdir().expect("home");
    let none = |_: &str| None;
    let added = |state: &devpit_rpc::DevflowMcp, id: &str| {
        state
            .tools
            .iter()
            .find(|one| one.tool == id)
            .expect("tool")
            .added
    };
    let state = state_in(home.path(), &none, CMD);
    assert_eq!(state.tools.len(), 4);
    assert!(!added(&state, "cursor"));

    for tool in TOOLS {
        set_in(home.path(), &none, tool, true, CMD).expect("added");
    }
    let state = state_in(home.path(), &none, CMD);
    assert!(state.tools.iter().all(|one| one.added), "{state:?}");

    set_in(home.path(), &none, Tool::Gemini, false, CMD).expect("removed");
    let state = state_in(home.path(), &none, CMD);
    assert!(!added(&state, "gemini"));
    assert!(added(&state, "codex"));
    set_in(home.path(), &none, Tool::Gemini, false, CMD).expect("removed twice is nothing");
}

/// These files hold the tools' tokens, and most are the person's alone: a
/// write keeps a file's permissions, its backup is no wider, and a new file
/// is theirs alone.
#[cfg(unix)]
#[test]
fn a_write_never_lets_anyone_else_read_a_tools_file() {
    use std::os::unix::fs::PermissionsExt;

    let mode = |path: &Path| std::fs::metadata(path).expect("meta").permissions().mode() & 0o777;
    let home = tempfile::tempdir().expect("home");
    let none = |_: &str| None;
    let file = Tool::Codex.file(home.path(), none);
    let dir = file.parent().expect("dir");
    std::fs::create_dir_all(dir).expect("mkdir");
    std::fs::write(&file, "TOKEN = \"secret\"\n").expect("theirs");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");

    set_in(home.path(), &none, Tool::Codex, true, CMD).expect("added");
    assert_eq!(mode(&file), 0o600);
    let backup = std::fs::read_dir(dir)
        .expect("dir")
        .flatten()
        .map(|entry| entry.path())
        .find(|path| path.to_string_lossy().contains("devpit-backup"))
        .expect("a backup");
    assert_eq!(mode(&backup), 0o600);
    assert_eq!(
        std::fs::read_to_string(&backup).expect("backup"),
        "TOKEN = \"secret\"\n"
    );

    set_in(home.path(), &none, Tool::Cursor, true, CMD).expect("added");
    assert_eq!(mode(&Tool::Cursor.file(home.path(), none)), 0o600);
}

/// A dotfiles link stays a link, and what it points at gets the change.
#[cfg(unix)]
#[test]
fn a_tools_file_linked_from_elsewhere_stays_a_link() {
    let home = tempfile::tempdir().expect("home");
    let none = |_: &str| None;
    let real = home.path().join("dotfiles-settings.json");
    std::fs::write(&real, "{}").expect("real");
    let file = Tool::Gemini.file(home.path(), none);
    std::fs::create_dir_all(file.parent().expect("dir")).expect("mkdir");
    std::os::unix::fs::symlink(&real, &file).expect("link");

    set_in(home.path(), &none, Tool::Gemini, true, CMD).expect("added");

    let meta = std::fs::symlink_metadata(&file).expect("meta");
    assert!(meta.file_type().is_symlink(), "the link was replaced");
    assert!(added_in(
        Tool::Gemini,
        &std::fs::read_to_string(&real).expect("real")
    ));
}
