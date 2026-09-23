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
