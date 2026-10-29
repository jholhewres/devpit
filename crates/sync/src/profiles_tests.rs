use super::*;

#[test]
fn only_what_names_an_account_folder_a_gateway_or_a_model_carries_a_value() {
    for name in [
        "CLAUDE_CONFIG_DIR",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_MODEL",
        "ANTHROPIC_SMALL_FAST_MODEL",
        "API_TIMEOUT_MS",
    ] {
        assert!(shared(name), "{name}");
    }
    for name in [
        "ANTHROPIC_AUTH_TOKEN",
        "ANTHROPIC_API_KEY",
        "GITHUB_TOKEN",
        "PASSWORD",
    ] {
        assert!(!shared(name), "{name}");
    }
}

#[test]
fn a_value_from_elsewhere_is_held_to_its_rules() {
    assert!(fits("env.ANTHROPIC_AUTH_TOKEN", Some("")));
    assert!(
        !fits("env.ANTHROPIC_AUTH_TOKEN", Some("sk-anything")),
        "a secret never arrives"
    );
    assert!(fits(
        "env.ANTHROPIC_BASE_URL",
        Some("https://api.z.ai/api/anthropic")
    ));
    assert!(!fits("env.ANTHROPIC_BASE_URL", Some("a\nb")));
    assert!(!fits("env.1BAD", Some("")));
    assert!(!fits("env.A-B", Some("")));
    assert!(fits(COMMAND, Some("claude")));
    assert!(fits(COMMAND, Some("")));
    assert!(!fits(COMMAND, Some("/usr/bin/claude")));
    assert!(!fits(COMMAND, Some("rm -rf")));
    assert!(
        !fits(COMMAND, Some("claude;id")),
        "the editor's rule: one word"
    );
    assert!(!fits(COMMAND, Some("claude$IFS")));
    assert!(fits(MODELS, Some(r#"["glm-5"]"#)));
    assert!(
        !fits("args", Some(r#"["--x","y"]"#)),
        "arguments stay where they were typed"
    );
    assert!(
        !fits("env.AZURE_MODEL_API_KEY", Some("x")),
        "a secret's name wins"
    );
    assert!(!fits("env.GW_BASE_URL", Some("https://u:p@h/x")));
    assert!(!fits("env.GW_BASE_URL", Some("https://h/x?key=1")));
    assert!(fits("env.ANTHROPIC_MODEL", None), "a removed variable");
    assert!(!fits(BASE, Some("../x")));
    assert!(!fits(LABEL, Some(" ")));
    assert!(!fits("other", Some("x")));
}

#[test]
fn an_empty_value_from_elsewhere_never_empties_a_secret_filled_here() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    store
        .set_preference(
            key::AGENT_PROFILES,
            &json!([{"id": "p1", "label": "GLM", "base": "claude", "command": "",
                     "args": [], "models": [],
                     "env": [{"name": "ANTHROPIC_AUTH_TOKEN", "value": "sk-HERE"}]}])
            .to_string(),
        )
        .expect("profiles");
    let space = Profiles {
        store: &store,
        home: dir.path(),
    };

    space
        .write(&identity("p1"), "env.ANTHROPIC_AUTH_TOKEN", Some(""))
        .expect("write");
    space
        .write(&identity("p1"), "env.OTHER_TOKEN", Some(""))
        .expect("write");

    let list = space.list().expect("list").expect("readable");
    assert_eq!(
        vars(&list[0]),
        vec![
            ("ANTHROPIC_AUTH_TOKEN".to_owned(), "sk-HERE".to_owned()),
            ("OTHER_TOKEN".to_owned(), String::new()),
        ]
    );
}

#[test]
fn a_profile_without_a_name_or_an_agent_is_not_carried() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    store
        .set_preference(
            key::AGENT_PROFILES,
            &json!([{"id": "half", "label": "", "base": "claude"},
                    {"id": "odd", "label": "Odd", "base": "../x"}])
            .to_string(),
        )
        .expect("profiles");
    let space = Profiles {
        store: &store,
        home: dir.path(),
    };

    assert!(space.here().expect("here").is_empty());
}

#[test]
fn a_profile_made_unstartable_here_still_asks_after_its_making_round() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let space = Profiles {
        store: &store,
        home: dir.path(),
    };
    let record = identity("p9");
    // A record with no agent: made here, but not one here() can carry.
    space
        .write(&record, COMMAND, Some("claude"))
        .expect("command");
    space.write(&record, LABEL, Some("GLM")).expect("label");
    let mut all = kept(&store);
    all.profiles.get_mut("p9").expect("held").arrived = false;
    keep(&store, &all).expect("taken");

    space.here().expect("the next round looks");
    space.write(&record, BASE, Some("claude")).expect("base");
    space
        .write(
            &record,
            "env.ANTHROPIC_BASE_URL",
            Some("https://attacker.example/api"),
        )
        .expect("base url");

    let held = kept(&store).profiles["p9"].clone();
    assert!(held
        .shadow
        .get("env.ANTHROPIC_BASE_URL")
        .is_some_and(|one| one.ask));
    assert!(held.shadow.get(BASE).is_some_and(|one| one.ask));
}
