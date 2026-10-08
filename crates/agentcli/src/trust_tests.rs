use std::path::Path;

use serde_json::{json, Map, Value};

use super::{key_of, trust_in, trust_with, trusted, trusts};

fn object(value: Value) -> Map<String, Value> {
    value.as_object().expect("an object").clone()
}

fn written(path: &Path) -> Map<String, Value> {
    object(serde_json::from_str(&std::fs::read_to_string(path).expect("read")).expect("json"))
}

#[test]
fn only_the_exact_path_is_trusted_never_its_parent() {
    let config =
        object(json!({ "projects": { "/home/a/work": { "hasTrustDialogAccepted": true } } }));
    assert!(trusts(&config, "/home/a/work"));
    assert!(!trusts(&config, "/home/a/work/repo"));
    assert!(!trusts(
        &object(json!({ "projects": { "/x": { "hasTrustDialogAccepted": false } } })),
        "/x"
    ));
}

#[test]
fn trusting_adds_the_one_key_and_keeps_the_rest_of_the_entry() {
    let mut config = object(json!({
        "numStartups": 7,
        "projects": { "/p": { "allowedTools": ["Bash"], "lastCost": 0.5 } }
    }));
    let keys = vec!["/p".to_owned(), "/w".to_owned()];
    assert_eq!(trusted(&mut config, &keys), Ok(true));
    assert_eq!(config["numStartups"], 7);
    assert_eq!(config["projects"]["/p"]["allowedTools"], json!(["Bash"]));
    assert!(trusts(&config, "/p") && trusts(&config, "/w"));
    // Already trusted: nothing to write.
    assert_eq!(trusted(&mut config, &keys), Ok(false));
}

#[test]
fn a_projects_that_is_not_an_object_is_refused_not_replaced() {
    let mut config = object(json!({ "projects": [1, 2] }));
    assert!(trusted(&mut config, &["/p".to_owned()]).is_err());
    assert_eq!(config["projects"], json!([1, 2]));
}

#[test]
fn an_account_that_never_ran_gets_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join(".claude.json");
    assert_eq!(trust_in(&file, &["/p".to_owned()]), Ok(false));
    assert!(!file.exists());
}

#[test]
fn a_file_that_is_not_json_is_left_as_it_was() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join(".claude.json");
    std::fs::write(&file, "{ half").expect("write");
    assert!(trust_in(&file, &["/p".to_owned()]).is_err());
    assert_eq!(std::fs::read_to_string(&file).expect("read"), "{ half");
}

#[test]
fn a_write_by_the_cli_meanwhile_is_kept_and_the_trust_still_lands() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join(".claude.json");
    std::fs::write(&file, r#"{"projects":{}}"#).expect("write");
    let mut first = true;
    let wrote = trust_with(&file, &["/p".to_owned()], || {
        if std::mem::take(&mut first) {
            std::fs::write(&file, r#"{"projects":{},"userID":"abc"}"#).expect("cli write");
        }
    });
    assert_eq!(wrote, Ok(true));
    let after = written(&file);
    assert_eq!(after["userID"], "abc");
    assert!(trusts(&after, "/p"));
    // No temporary file is left behind, written or abandoned.
    let left: Vec<_> = std::fs::read_dir(dir.path())
        .expect("dir")
        .flatten()
        .collect();
    assert_eq!(left.len(), 1);
}

#[test]
fn a_file_that_never_stops_changing_is_given_up_on_untouched() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join(".claude.json");
    std::fs::write(&file, r#"{"n":0}"#).expect("write");
    let mut n = 0;
    let wrote = trust_with(&file, &["/p".to_owned()], || {
        n += 1;
        std::fs::write(&file, format!(r#"{{"n":{n}}}"#)).expect("cli write");
    });
    assert!(wrote.is_err());
    assert!(!trusts(&written(&file), "/p"));
}

#[cfg(unix)]
#[test]
fn the_file_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join(".claude.json");
    std::fs::write(&file, "{}").expect("write");
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).expect("chmod");
    assert_eq!(trust_in(&file, &["/p".to_owned()]), Ok(true));
    let mode = std::fs::metadata(&file).expect("meta").permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[cfg(unix)]
#[test]
fn a_key_is_the_path_as_it_is() {
    assert_eq!(key_of(Path::new("/home/a/my repo")), "/home/a/my repo");
}
