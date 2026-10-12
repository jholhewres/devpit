use devpit_devflow::Ask;
use serde_json::Value;

use super::*;

const EXE: &str = "/opt/devpit";

#[test]
fn claude_code_gets_the_plugin_folder_for_its_session_only() {
    let root = tempfile::tempdir().expect("root");
    let flag = flags_for(Cli::Claude, root.path(), EXE).expect("a flag");
    let base = folder(root.path(), Cli::Claude);
    assert_eq!(flag, format!("--plugin-dir='{}'", base.display()));
    let hooks: Value = serde_json::from_str(
        &std::fs::read_to_string(base.join("hooks/hooks.json")).expect("hooks"),
    )
    .expect("json");
    assert_eq!(
        hooks["hooks"]["Stop"][0]["hooks"][0]["tool"],
        "devpit_devflow_gate"
    );
    assert!(base.join(".claude-plugin/plugin.json").is_file());
    assert!(base.join("skills/completion/SKILL.md").is_file());
}

#[cfg(not(windows))]
#[test]
fn codex_gets_its_hooks_and_their_trust_on_the_line_and_its_skills_as_a_marketplace() {
    let root = tempfile::tempdir().expect("root");
    let flag = flags_for(Cli::Codex, root.path(), EXE).expect("a flag");
    assert!(flag.starts_with("-c 'hooks={"), "{flag}");
    assert!(flag.contains("trusted_hash="), "{flag}");
    let base = folder(root.path(), Cli::Codex);
    assert!(base.join(".agents/plugins/marketplace.json").is_file());
    assert!(base
        .join("devflow/skills/devflow-completion/SKILL.md")
        .is_file());
}

/// The two crates name the same tool and the same cap.
#[test]
fn the_hooks_call_the_tool_the_server_lists_and_follow_the_same_cap() {
    assert_eq!(
        devpit_agentapi::devflow::GATE_TOOL,
        devpit_devflow::plugin::GATE_TOOL
    );
    assert_eq!(
        devpit_agentapi::devflow::CAP_ENV,
        devpit_devflow::gate::CAP_ENV
    );
}

/// Switched on, a session of the project starts with Devflow's hooks;
/// switched off, the next one starts without them.
#[test]
fn switching_devflow_off_takes_the_hooks_off_the_next_launch() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().canonicalize().expect("root");
    let store = crate::projects::store().expect("store");
    let id = store.add_project(&root, None).expect("project");
    std::fs::create_dir_all(root.join(".devpit/devflow")).expect("folder");
    std::fs::write(
        root.join(".devpit/devflow/devflow.yaml"),
        "version: 1\nenabled: true\n",
    )
    .expect("yaml");
    assert_eq!(
        flags("claude", &id),
        None,
        "the file alone waits for the click"
    );

    crate::devflow_asks::remember(&store, &id, Ask::Enable, true).expect("clicked");
    let on = flags("claude", &id).expect("on");
    assert!(on.starts_with("--plugin-dir="), "{on}");
    assert_eq!(
        flags("gemini", &id),
        None,
        "no hooks for a CLI devflow has none for"
    );

    crate::devflow_asks::remember(&store, &id, Ask::Enable, false).expect("off");
    assert_eq!(flags("claude", &id), None);
}
