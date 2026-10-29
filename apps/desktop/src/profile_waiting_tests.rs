//! A profile from another machine waits for the person before it starts.

use devpit_core::Store;
use devpit_rpc::{Declared, EnvVar};
use devpit_sync::profiles::{keep, kept, Held, Shadow};

use super::*;

fn glm(token: &str) -> Declared {
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
                value: token.to_owned(),
            },
        ],
        models: Vec::new(),
    }
}

fn store(token: &str, held: Held) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    crate::agent_profiles::save(&store, &[glm(token)]).expect("save");
    let mut all = kept(&store);
    all.profiles.insert("p1".to_owned(), held);
    keep(&store, &all).expect("kept");
    (dir, store)
}

fn asked(field: &str, value: &str) -> Held {
    let mut held = Held::default();
    held.shadow.insert(
        field.to_owned(),
        Shadow {
            value: value.to_owned(),
            ask: true,
        },
    );
    held
}

#[test]
fn a_profile_missing_a_secret_that_arrived_cannot_start_and_says_which() {
    let mut held = Held::default();
    held.needs.insert("ANTHROPIC_AUTH_TOKEN".to_owned());
    let (_dir, store) = store("", held);

    let why = blocked(&store, "p1").expect("blocked");

    assert!(why.contains("ANTHROPIC_AUTH_TOKEN"), "{why}");
    assert!(crate::agent_profiles::runner_for(&store, "p1")
        .err()
        .is_some_and(|err| err.contains("ANTHROPIC_AUTH_TOKEN")));
}

#[test]
fn a_variable_left_empty_on_purpose_here_blocks_nothing() {
    let (_dir, store) = store("", Held::default());

    assert_eq!(blocked(&store, "p1"), None);
}

#[test]
fn a_profile_that_arrived_waits_for_a_yes() {
    let held = Held {
        arrived: true,
        ..Held::default()
    };
    let (_dir, store) = store("sk-<REDACTED>", held);

    assert!(blocked(&store, "p1").is_some_and(|why| why.contains("another computer")));

    settle(&store, "p1", None, None, true).expect("take");

    assert_eq!(blocked(&store, "p1"), None);
}

#[test]
fn a_base_url_changed_elsewhere_waits_until_taken() {
    let (_dir, store) = store(
        "sk-<REDACTED>",
        asked("env.ANTHROPIC_BASE_URL", "https://elsewhere.example/api"),
    );

    let why = blocked(&store, "p1").expect("blocked");
    assert!(why.contains("ANTHROPIC_BASE_URL"), "{why}");

    assert!(
        settle(
            &store,
            "p1",
            Some("env.ANTHROPIC_BASE_URL"),
            Some("https://swapped.example/api"),
            true
        )
        .is_err(),
        "a value the person did not see is not taken"
    );
    settle(
        &store,
        "p1",
        Some("env.ANTHROPIC_BASE_URL"),
        Some("https://elsewhere.example/api"),
        true,
    )
    .expect("take");

    assert!(blocked(&store, "p1").is_none());
    assert_eq!(
        crate::agent_profiles::declared(&store)[0].env[0].value,
        "https://elsewhere.example/api"
    );
}

#[test]
fn keeping_this_machines_own_clears_the_wait_and_changes_nothing() {
    let (_dir, store) = store("sk-<REDACTED>", asked("command", "sh"));

    settle(&store, "p1", Some("command"), Some("sh"), false).expect("keep");

    assert!(waiting(&store).is_empty());
    assert_eq!(crate::agent_profiles::declared(&store)[0].command, "");
}

#[test]
fn keeping_out_a_base_url_this_machine_never_had_stays_kept_out() {
    let (_dir, store) = store(
        "sk-<REDACTED>",
        asked("env.OTHER_BASE_URL", "https://elsewhere.example/api"),
    );

    settle(
        &store,
        "p1",
        Some("env.OTHER_BASE_URL"),
        Some("https://elsewhere.example/api"),
        false,
    )
    .expect("keep");

    assert!(waiting(&store).is_empty());
    let held = kept(&store).profiles["p1"].clone();
    assert!(
        !held.shadow["env.OTHER_BASE_URL"].ask,
        "beside, unused, not asked again"
    );
}

#[test]
fn the_reviewers_do_not_run_on_another_account_while_theirs_waits() {
    let (_dir, store) = store(
        "sk-<REDACTED>",
        asked("env.ANTHROPIC_BASE_URL", "https://elsewhere.example/api"),
    );
    let project = store.add_project(_dir.path(), None).expect("project");
    crate::devflow_profile::set_named(&store, &project, Some("p1")).expect("named");

    let refused = crate::devflow_profile::runnable(&store, &project, None);

    assert!(refused.is_err_and(|why| why.contains("ANTHROPIC_BASE_URL")));
}

#[test]
fn a_profile_that_arrived_is_not_taken_while_a_change_to_it_still_asks() {
    let mut held = asked("env.ANTHROPIC_BASE_URL", "https://elsewhere.example/api");
    held.arrived = true;
    let (_dir, store) = store("sk-<REDACTED>", held);

    assert!(settle(&store, "p1", None, None, true).is_err());
    assert!(blocked(&store, "p1").is_some());
}
