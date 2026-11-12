use super::*;

fn profile(installed: Option<&str>, enabled: Option<bool>) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("dir");
    if let Some(version) = installed {
        std::fs::create_dir_all(dir.path().join("plugins")).expect("plugins");
        let installed = serde_json::json!({ "version": 2, "plugins": {
            "devflow@devflow": [{ "scope": "user", "version": version }],
            "other@else": [{ "scope": "user", "version": "9.9.9" }],
        }});
        std::fs::write(
            dir.path().join("plugins/installed_plugins.json"),
            installed.to_string(),
        )
        .expect("installed");
    }
    if let Some(on) = enabled {
        let settings =
            serde_json::json!({ "enabledPlugins": { "devflow@devflow": on, "other@else": true } });
        std::fs::write(dir.path().join("settings.json"), settings.to_string()).expect("settings");
    }
    dir
}

#[test]
fn the_plugin_is_read_off_the_profiles_own_files() {
    let missing = profile(None, None);
    assert_eq!(read_in(missing.path()), Read::default());
    let on = profile(Some("0.2.1"), Some(true));
    assert_eq!(
        read_in(on.path()),
        Read {
            version: Some("0.2.1".into()),
            enabled: true
        }
    );
    // As 03/11 left the default profile: installed, switched off.
    let off = profile(Some("0.1.0"), Some(false));
    assert_eq!(
        read_in(off.path()),
        Read {
            version: Some("0.1.0".into()),
            enabled: false
        }
    );
}

#[test]
fn the_plugin_on_beside_devpits_devflow_is_the_tools_twice() {
    let on = read_in(profile(Some("0.2.1"), Some(true)).path());
    let off = read_in(profile(Some("0.2.1"), Some(false)).path());
    let gone = read_in(profile(None, Some(true)).path());
    assert!(twice(&on, true));
    assert!(!twice(&on, false));
    assert!(!twice(&off, true));
    assert!(!twice(&gone, true));
}

/// A `claude` that writes its arguments and the directory it was pointed at.
#[cfg(unix)]
fn fake_claude(log: &Path) -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let bin = tempfile::tempdir().expect("bin");
    let script = format!(
        "#!/bin/sh\nprintf '%s|%s\\n' \"${{CLAUDE_CONFIG_DIR:-default}}\" \"$*\" >> '{}'\n",
        log.display()
    );
    let path = bin.path().join("claude");
    std::fs::write(&path, script).expect("script");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    bin
}

#[cfg(unix)]
#[test]
fn install_update_and_remove_run_claude_in_the_projects_profile_and_nowhere_else() {
    let log_dir = tempfile::tempdir().expect("log");
    let log = log_dir.path().join("calls");
    let bin = fake_claude(&log);
    let program = bin.path().join("claude");
    let home = tempfile::tempdir().expect("home");
    let named = home.path().join(".claude-work");
    for action in [
        DevflowPluginAction::Install,
        DevflowPluginAction::Update,
        DevflowPluginAction::Disable,
        DevflowPluginAction::Remove,
    ] {
        run(&program, &named, home.path(), action).expect("ran");
    }
    let calls = std::fs::read_to_string(&log).expect("log");
    let at = named.display();
    assert_eq!(
        calls,
        format!(
            "{at}|plugin marketplace add jholhewres/devflow\n\
             {at}|plugin marketplace update devflow\n\
             {at}|plugin install devflow@devflow\n\
             {at}|plugin marketplace update devflow\n\
             {at}|plugin update devflow@devflow\n\
             {at}|plugin disable devflow@devflow\n\
             {at}|plugin uninstall devflow@devflow\n"
        )
    );
    // The default profile is the CLI's own default: nothing to point it at.
    std::fs::remove_file(&log).expect("reset");
    run(
        &program,
        &home.path().join(".claude"),
        home.path(),
        DevflowPluginAction::Enable,
    )
    .expect("ran");
    assert_eq!(
        std::fs::read_to_string(&log).expect("log"),
        "default|plugin enable devflow@devflow\n"
    );
}

#[cfg(unix)]
#[test]
fn a_failed_step_says_which_and_stops_the_rest() {
    use std::os::unix::fs::PermissionsExt;
    let bin = tempfile::tempdir().expect("bin");
    let program = bin.path().join("claude");
    std::fs::write(&program, "#!/bin/sh\necho 'no such plugin' >&2\nexit 1\n").expect("script");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let home = tempfile::tempdir().expect("home");
    let err = run(
        &program,
        &home.path().join(".claude"),
        home.path(),
        DevflowPluginAction::Remove,
    )
    .expect_err("failed");
    assert!(
        err.message
            .contains("`claude plugin uninstall devflow@devflow` failed: no such plugin"),
        "{}",
        err.message
    );
}

#[test]
fn the_known_release_is_the_engines_tag() {
    let manifest =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.toml"))
            .expect("manifest");
    let line = manifest
        .lines()
        .find(|line| line.starts_with("devflow-engine"))
        .expect("engine line");
    assert!(
        line.contains(&format!("tag = \"v{KNOWN}\"")),
        "devflow-engine is at another tag than {KNOWN}: {line}"
    );
}

/// The real CLI and marketplace, in a throwaway profile under
/// `DEVPIT_PROBE_DIR`: needs the network, so `make test` never runs it.
#[test]
#[ignore = "needs claude and the network"]
fn the_real_plugin_is_installed_updated_disabled_and_removed_in_a_throwaway_profile() {
    let base = std::env::var_os("DEVPIT_PROBE_DIR").expect("DEVPIT_PROBE_DIR");
    let probe = tempfile::tempdir_in(base).expect("probe");
    let home = probe.path().join("home");
    let profile = probe.path().join("profile");
    std::fs::create_dir_all(&home).expect("home");
    let claude = Path::new("claude");
    run(claude, &profile, &home, DevflowPluginAction::Install).expect("install");
    let installed = read_in(&profile);
    assert_eq!(installed.version.as_deref(), Some(KNOWN));
    assert!(installed.enabled && twice(&installed, true));
    run(claude, &profile, &home, DevflowPluginAction::Update).expect("update");
    run(claude, &profile, &home, DevflowPluginAction::Disable).expect("disable");
    assert!(!twice(&read_in(&profile), true));
    run(claude, &profile, &home, DevflowPluginAction::Remove).expect("remove");
    assert_eq!(read_in(&profile), Read::default());
}
