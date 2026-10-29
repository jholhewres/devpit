//! A profile from another machine waits for the person before it starts.

use devpit_core::{preference, Store};
use devpit_rpc::{Declared, EnvVar};

use super::*;

fn glm() -> Declared {
    Declared {
        id: "p1".to_owned(),
        label: "GLM".to_owned(),
        base: "claude".to_owned(),
        command: String::new(),
        args: Vec::new(),
        env: vec![
            EnvVar {
                name: "ANTHROPIC_BASE_URL".to_owned(),
                value: "https://api.z.ai/api/anthropic".to_owned(),
            },
            EnvVar {
                name: "ANTHROPIC_AUTH_TOKEN".to_owned(),
                value: String::new(),
            },
        ],
        models: Vec::new(),
    }
}

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    crate::agent_profiles::save(&store, &[glm()]).expect("save");
    (dir, store)
}

#[test]
fn a_profile_missing_a_secret_cannot_start_and_says_which() {
    let (_dir, store) = store();

    let why = blocked(&store, "p1").expect("blocked");

    assert!(why.contains("ANTHROPIC_AUTH_TOKEN"), "{why}");
    assert!(crate::agent_profiles::runner_for(&store, "p1")
        .err()
        .is_some_and(|err| err.contains("ANTHROPIC_AUTH_TOKEN")));
}

#[test]
fn a_base_url_changed_elsewhere_waits_until_taken_or_kept() {
    let (_dir, store) = store();
    let mut filled = glm();
    filled.env[1].value = "sk-<REDACTED>".to_owned();
    crate::agent_profiles::save(&store, &[filled]).expect("save");
    store
        .set_preference(
            preference::AGENT_PROFILES_PENDING,
            r#"{"p1":{"ANTHROPIC_BASE_URL":"https://elsewhere.example/api"}}"#,
        )
        .expect("pending");

    let why = blocked(&store, "p1").expect("blocked");
    assert!(why.contains("ANTHROPIC_BASE_URL"), "{why}");

    settle(&store, "p1", true).expect("take");

    assert!(blocked(&store, "p1").is_none());
    let taken = crate::agent_profiles::declared(&store);
    assert_eq!(taken[0].env[0].value, "https://elsewhere.example/api");
}

#[test]
fn keeping_this_machines_base_url_clears_the_wait_and_changes_nothing() {
    let (_dir, store) = store();
    let mut filled = glm();
    filled.env[1].value = "sk-<REDACTED>".to_owned();
    crate::agent_profiles::save(&store, &[filled]).expect("save");
    store
        .set_preference(
            preference::AGENT_PROFILES_PENDING,
            r#"{"p1":{"ANTHROPIC_BASE_URL":"https://elsewhere.example/api"}}"#,
        )
        .expect("pending");

    settle(&store, "p1", false).expect("keep");

    assert!(waiting(&store).is_empty());
    assert_eq!(
        crate::agent_profiles::declared(&store)[0].env[0].value,
        "https://api.z.ai/api/anthropic"
    );
}
