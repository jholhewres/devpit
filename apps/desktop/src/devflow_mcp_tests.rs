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

/// The screen promises the rest of a hand-kept file is left as it is: its
/// order, its indentation and its line ends. Taking devflow out gives the
/// file back byte for byte.
#[test]
fn a_json_file_keeps_its_order_and_its_spacing() {
    let theirs = "{\n    \"theme\": \"dark\",\n    \"mcpServers\": {\n        \"other\": { \"command\": \"x\" }\n    },\n    \"aaa\": 1\n}\n";
    for tool in [Tool::Cursor, Tool::Gemini] {
        let added = with(tool, Some(theirs), CMD).expect("added");
        assert_eq!(
            added,
            format!(
                "{{\n    \"theme\": \"dark\",\n    \"mcpServers\": {{\n        \"other\": {{ \"command\": \"x\" }},\n        \"devflow\": {{\n            \"args\": [\n                \"mcp\"\n            ],\n            \"command\": \"{CMD}\"\n        }}\n    }},\n    \"aaa\": 1\n}}\n"
            )
        );
        assert_eq!(without(tool, &added).expect("removed"), theirs);
    }
    let crlf = theirs.replace('\n', "\r\n");
    let added = with(Tool::Cursor, Some(&crlf), CMD).expect("added");
    assert!(!added.replace("\r\n", "").contains('\n'), "{added:?}");
    assert_eq!(without(Tool::Cursor, &added).expect("removed"), crlf);
}

#[test]
fn a_json_file_without_servers_gets_them_last_in_its_own_indentation() {
    let theirs = "{\n\t\"theme\": \"dark\"\n}";
    let added = with(Tool::Gemini, Some(theirs), CMD).expect("added");
    assert_eq!(
        added,
        format!("{{\n\t\"theme\": \"dark\",\n\t\"mcpServers\": {{\n\t\t\"devflow\": {{\n\t\t\t\"args\": [\n\t\t\t\t\"mcp\"\n\t\t\t],\n\t\t\t\"command\": \"{CMD}\"\n\t\t}}\n\t}}\n}}")
    );
    let compact = r#"{"mcpServers":{"other":{"command":"x"}}}"#;
    assert_eq!(
        with(Tool::Cursor, Some(compact), CMD).expect("added"),
        format!(
            r#"{{"mcpServers":{{"other":{{"command":"x"}},"devflow":{{"args":["mcp"],"command":"{CMD}"}}}}}}"#
        )
    );
}

/// Notepad saves a byte-order mark: the file it wrote reads as added, and
/// Remove finds what Add put there.
#[test]
fn a_json_file_with_a_byte_order_mark_reads_as_added_and_can_be_removed() {
    let theirs = "\u{feff}{\"theme\": \"dark\"}";
    let added = with(Tool::Cursor, Some(theirs), CMD).expect("added");
    assert!(added_in(Tool::Cursor, &added), "{added}");
    let removed = without(Tool::Cursor, &added).expect("removed");
    assert!(!added_in(Tool::Cursor, &removed), "{removed}");
    assert!(removed.starts_with('\u{feff}'), "{removed}");
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

fn codex_command(text: &str) -> Option<String> {
    let doc: toml_edit::DocumentMut = text.parse().unwrap_or_else(|err| panic!("{text}\n{err}"));
    doc.get("mcp_servers")?
        .get(NAME)?
        .get("command")?
        .as_str()
        .map(str::to_owned)
}

/// Valid TOML for the same server, however the person spelled it.
const CODEX_SPELLINGS: [&str; 5] = [
    "[mcp_servers.devflow] # mine\ncommand = \"x\"\n",
    "[mcp_servers.\"devflow\"]\ncommand = \"x\"\n",
    "[ mcp_servers . devflow ]\ncommand = \"x\"\n",
    "[mcp_servers]\ndevflow = { command = \"x\" }\n",
    "mcp_servers.devflow.command = \"x\"\n",
];

/// Codex refuses a config.toml that declares a table twice, so an entry
/// missed for its spelling must never be added again beside it.
#[test]
fn a_codex_entry_spelled_another_way_is_found_and_never_added_twice() {
    for theirs in CODEX_SPELLINGS {
        assert!(added_in(Tool::Codex, theirs), "{theirs}");
        let again = with(Tool::Codex, Some(theirs), CMD).expect("added");
        assert_eq!(codex_command(&again).as_deref(), Some(CMD), "{again}");
        let removed = without(Tool::Codex, theirs).expect("removed");
        assert!(!added_in(Tool::Codex, &removed), "{removed}");
        assert_eq!(codex_command(&removed), None, "{removed}");
    }
}

#[test]
fn a_codex_file_that_is_not_toml_is_left_alone() {
    let broken = "[mcp_servers.devflow]\ncommand = \"x\"\n[mcp_servers.devflow]\n";
    assert!(!added_in(Tool::Codex, broken));
    assert!(with(Tool::Codex, Some(broken), CMD).is_err());
    assert!(without(Tool::Codex, broken).is_err());
}

/// What sits around devflow's table is the person's: a comment above the next
/// table, a comment at the end of a line, the file's own line ends.
#[test]
fn taking_codex_out_keeps_every_comment_that_is_not_its_own() {
    let theirs = "model = \"o3\" # fast\n\n[mcp_servers.devflow]\ncommand = \"x\"\n\n# my review profile, do not delete\n[profiles.x]\nmodel = \"m\"\n";
    let removed = without(Tool::Codex, theirs).expect("removed");
    assert_eq!(
        removed,
        "model = \"o3\" # fast\n\n# my review profile, do not delete\n[profiles.x]\nmodel = \"m\"\n"
    );
    let crlf = theirs.replace('\n', "\r\n");
    let added = with(Tool::Codex, Some(&crlf), CMD).expect("added");
    assert!(!added.replace("\r\n", "").contains('\n'), "{added:?}");
    assert_eq!(
        without(Tool::Codex, &crlf).expect("removed"),
        removed.replace('\n', "\r\n")
    );
}

#[test]
fn codex_gets_its_own_table_and_loses_only_that_one() {
    let theirs = "model = \"o3\"\n\n[mcp_servers.other]\ncommand = \"x\"\n";
    let added = with(Tool::Codex, Some(theirs), r"C:\bin\devflow").expect("added");
    assert!(added.starts_with(theirs), "{added}");
    assert!(added.contains("\n\n[mcp_servers.devflow]\n"), "{added}");
    assert_eq!(codex_command(&added).as_deref(), Some(r"C:\bin\devflow"));
    assert!(added_in(Tool::Codex, &added));
    assert_eq!(without(Tool::Codex, &added).expect("removed"), theirs);
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

/// One copy of the file as it was before the last write, under a name the
/// person can find, not one more file beside theirs for every click.
#[test]
fn a_tools_file_keeps_one_backup_beside_it() {
    let home = tempfile::tempdir().expect("home");
    let none = |_: &str| None;
    let file = Tool::Cursor.file(home.path(), none);
    let dir = file.parent().expect("dir");
    std::fs::create_dir_all(dir).expect("mkdir");
    std::fs::write(&file, "{}\n").expect("theirs");

    set_in(home.path(), &none, Tool::Cursor, true, CMD).expect("added");
    set_in(home.path(), &none, Tool::Cursor, false, CMD).expect("removed");
    let before_last = std::fs::read_to_string(&file).expect("removed");
    set_in(home.path(), &none, Tool::Cursor, true, CMD).expect("added again");

    let backups: Vec<_> = std::fs::read_dir(dir)
        .expect("dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.to_string_lossy().contains("devpit-backup"))
        .collect();
    assert_eq!(backups, [dir.join("mcp.json.devpit-backup")]);
    assert_eq!(
        std::fs::read_to_string(&backups[0]).expect("backup"),
        before_last
    );
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
