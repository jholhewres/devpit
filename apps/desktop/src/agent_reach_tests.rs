use super::*;

#[test]
fn claude_is_given_a_config_file_it_reads_for_this_session_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let config = dir.path().join("mcp.json");
    let flags = mcp_flags("claude", Path::new("/opt/devpit"), &config).expect("flags");
    assert_eq!(flags, format!("--mcp-config '{}'", config.display()));
    let written: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config).expect("read")).expect("json");
    assert_eq!(written["mcpServers"]["devpit"]["command"], "/opt/devpit");
    assert_eq!(written["mcpServers"]["devpit"]["args"][0], "mcp");
    assert_eq!(
        written["mcpServers"]["devpit"]["env"]["DEVPIT_AGENT_ID"],
        "claude"
    );
}

#[test]
fn codex_is_given_the_server_as_overrides_on_its_own_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    let flags = mcp_flags(
        "codex",
        Path::new("/opt/devpit"),
        &dir.path().join("mcp.json"),
    )
    .expect("flags");
    assert!(
        flags.contains(r#"-c 'mcp_servers.devpit.command="/opt/devpit"'"#),
        "{flags}"
    );
    assert!(
        flags.contains(r#"mcp_servers.devpit.args=["mcp"]"#),
        "{flags}"
    );
    assert!(flags.contains(r#"DEVPIT_AGENT_ID="codex""#), "{flags}");
}

#[test]
fn an_agent_with_no_way_to_be_given_it_gets_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        mcp_flags(
            "gemini",
            Path::new("/opt/devpit"),
            &dir.path().join("m.json")
        ),
        None
    );
}

/// A quote in the path would end the shell word early and run the rest.
#[test]
fn a_path_it_cannot_quote_is_not_carried() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        mcp_flags(
            "codex",
            Path::new("/opt/it's/devpit"),
            &dir.path().join("m.json")
        ),
        None
    );
}

#[test]
fn the_shim_runs_this_binary_as_the_agent_cli() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = cli_shim(dir.path(), Path::new("/opt/devpit")).expect("shim");
    let script = std::fs::read_to_string(bin.join("devpit-agent")).expect("read");
    assert_eq!(script, "#!/bin/sh\nexec '/opt/devpit' agent \"$@\"\n");
}
