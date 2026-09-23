use super::*;

fn written(dir: &Path, installed: serde_json::Value) {
    let plugins = dir.join("plugins");
    std::fs::create_dir_all(&plugins).expect("dir");
    std::fs::write(
        plugins.join("installed_plugins.json"),
        installed.to_string(),
    )
    .expect("write");
}

#[test]
fn every_file_of_the_plugin_is_json_and_names_one_version() {
    let (version, files) = bundle(Path::new("/home/someone/.devpit"), "/opt/devpit");
    assert_eq!(files.len(), 4);
    for (path, text) in &files {
        serde_json::from_str::<serde_json::Value>(text)
            .unwrap_or_else(|err| panic!("{}: {err}", path.display()));
    }
    let find = |name: &str| {
        let text = &files
            .iter()
            .find(|(path, _)| path.ends_with(name))
            .expect(name)
            .1;
        serde_json::from_str::<serde_json::Value>(text).expect("json")
    };
    assert_eq!(find("plugin.json")["version"], version.as_str());
    assert_eq!(
        find("marketplace.json")["plugins"][0]["version"],
        version.as_str()
    );
    let server = &find(".mcp.json")["mcpServers"]["devpit"];
    assert_eq!(server["command"], "/opt/devpit");
    assert_eq!(server["env"][devpit_agentapi::mcp::FROM_PLUGIN], "1");
}

#[test]
fn the_version_follows_what_the_plugin_carries() {
    let (one, _) = bundle(Path::new("/a/.devpit"), "/opt/devpit");
    let (same, _) = bundle(Path::new("/a/.devpit"), "/opt/devpit");
    let (moved, _) = bundle(Path::new("/a/.devpit"), "/usr/bin/devpit");
    assert_eq!(one, same);
    assert_ne!(one, moved);
}

#[test]
fn an_installation_is_missing_behind_or_current() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(state_in(dir.path(), "1.0.0-b1"), ClaudePluginState::Missing);

    written(
        dir.path(),
        serde_json::json!({ "version": 2, "plugins": { "other@x": [{ "version": "1" }] } }),
    );
    assert_eq!(state_in(dir.path(), "1.0.0-b1"), ClaudePluginState::Missing);

    written(
        dir.path(),
        serde_json::json!({ "version": 2, "plugins": { KEY: [{ "version": "1.0.0-b0" }] } }),
    );
    assert_eq!(
        state_in(dir.path(), "1.0.0-b1"),
        ClaudePluginState::Outdated
    );

    written(
        dir.path(),
        serde_json::json!({ "version": 2, "plugins": { KEY: [{ "version": "1.0.0-b1" }] } }),
    );
    assert_eq!(state_in(dir.path(), "1.0.0-b1"), ClaudePluginState::Current);
}

fn found(directory: &str) -> Found {
    Found {
        directory: PathBuf::from(directory),
        said: None,
        profiles: Vec::new(),
        ids: Vec::new(),
        default: false,
    }
}

#[test]
fn every_installation_is_tried_and_every_failure_is_named() {
    let tried = std::cell::Cell::new(0);
    let failed = in_every(&[found("/a"), found("/b"), found("/c")], |one| {
        tried.set(tried.get() + 1);
        if one.directory == Path::new("/b") {
            Ok(())
        } else {
            Err(RpcError::internal("no network"))
        }
    })
    .expect_err("two failed");
    assert_eq!(tried.get(), 3);
    assert!(
        failed.message.contains("/a: no network") && failed.message.contains("/c: no network"),
        "{}",
        failed.message
    );
    assert!(!failed.message.contains("/b"), "{}", failed.message);
    assert!(in_every(&[found("/a")], |_| Ok(())).is_ok());
}

#[cfg(unix)]
#[test]
fn a_call_that_hangs_is_abandoned() {
    let mut command = std::process::Command::new("sh");
    command.args(["-c", "sleep 30"]);
    let started = std::time::Instant::now();
    let why = output_within(command, std::time::Duration::from_millis(300)).expect_err("hung");
    assert!(why.contains("did not finish"), "{why}");
    assert!(started.elapsed() < std::time::Duration::from_secs(5));
}

#[cfg(unix)]
#[test]
fn a_call_that_ends_says_what_it_said() {
    let mut command = std::process::Command::new("sh");
    command.args(["-c", "echo out; echo err >&2; exit 3"]);
    let out = output_within(command, std::time::Duration::from_secs(10)).expect("ran");
    assert_eq!(out.status.code(), Some(3));
    assert_eq!(out.stdout, b"out\n");
    assert_eq!(out.stderr, b"err\n");
}
