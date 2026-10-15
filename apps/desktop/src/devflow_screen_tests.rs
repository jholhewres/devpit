use super::*;

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    (dir, store)
}

fn project(id: &str, root: &Path) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": null,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("Makefile"), "test:\n\ttrue\n").expect("makefile");
    dir
}

fn write(root: &Path, text: &str) {
    std::fs::create_dir_all(root.join(".devpit/devflow")).expect("folder");
    std::fs::write(root.join(FILE), text).expect("devflow.yaml");
}

#[test]
fn the_screen_follows_the_file_through_its_four_states() {
    let (_home, store) = store();
    let dir = repo();
    let here = project("prj_screen_states", dir.path());

    let missing = screen(None, &store, &here);
    assert_eq!(missing.state, DevflowState::Missing);
    let detected = missing.detected.expect("what a setup starts from");
    assert_eq!(detected.found[0].command, "make test");
    assert_eq!(detected.found[0].source, "Makefile:1");
    let execution = &detected.recommended.stages[3];
    assert_eq!(
        (execution.name.as_str(), execution.checks[0].run.as_str()),
        ("execution", "make test")
    );
    assert_eq!(
        execution.checks[0].source.as_deref(),
        Some("Makefile:1"),
        "a recommended check says where its command was found"
    );

    write(
        dir.path(),
        "version: 1\nstages:\n  context: { enabled: false }\n",
    );
    let disabled = screen(None, &store, &here);
    assert_eq!(disabled.state, DevflowState::Disabled);
    assert!(disabled.detected.is_none(), "detection is only for a setup");
    let config = disabled.config.expect("the file's configuration");
    assert!(!config.stages[1].enabled, "context is switched off");
    assert_eq!(config.stages[2].approve.as_deref(), Some("person"));
    assert_eq!(
        config.stages[2]
            .skill
            .as_ref()
            .map(|skill| skill.layer.as_str()),
        Some("built-in")
    );

    write(dir.path(), "version: 1\nenabled: true\n");
    asks::remember(&store, &here.id, Ask::Enable, true).expect("clicked");
    assert_eq!(screen(None, &store, &here).state, DevflowState::Active);

    write(dir.path(), "version: 1\nenabled: maybe\n");
    let invalid = screen(None, &store, &here);
    assert_eq!(invalid.state, DevflowState::Invalid);
    let error = invalid.error.expect("why");
    assert_eq!((error.line, error.field.as_str()), (Some(2), "enabled"));
    assert_eq!(
        invalid.yaml.as_deref(),
        Some("version: 1\nenabled: maybe\n")
    );
}

#[test]
fn one_click_recommended_setup_writes_what_devpit_devflow_setup_writes() {
    let (_home, store) = store();
    let (screen_dir, tool_dir) = (repo(), repo());
    let on_screen = project("prj_screen_recommended", screen_dir.path());
    let by_tool = project("prj_tool_recommended", tool_dir.path());

    setup_now(
        None,
        &store,
        &on_screen,
        &setup_params("recommended", "{}", true).expect("params"),
    )
    .expect("created");
    for apply in [false, true] {
        devflow::respond(
            None,
            "devflow_setup",
            &by_tool,
            tool_dir.path(),
            &json!({ "preset": "recommended", "apply": apply }),
        )
        .expect("set up");
    }

    let read = |dir: &Path| std::fs::read_to_string(dir.join(FILE)).expect("yaml");
    assert_eq!(read(screen_dir.path()), read(tool_dir.path()));
    assert!(screen_dir
        .path()
        .join(".devpit/devflow/scripts/check-unit.sh")
        .is_file());
}

#[test]
fn an_edit_on_the_screen_writes_the_file_and_a_change_outside_is_what_it_reads() {
    let (_home, store) = store();
    let dir = repo();
    let here = project("prj_screen_edit", dir.path());
    write(
        dir.path(),
        "version: 1\n# kept\nreview:\n  consolidate: jev\n",
    );

    edit_now(None, &store, &here, "review.consolidate", json!("none")).expect("edited");
    let text = std::fs::read_to_string(dir.path().join(FILE)).expect("yaml");
    assert!(
        text.contains("# kept"),
        "a word's edit keeps the person's comments: {text}"
    );
    edit_now(None, &store, &here, "review.block_on", json!(["critical"])).expect("edited");
    let config = screen(None, &store, &here).config.expect("valid");
    assert_eq!(
        (config.consolidate.as_str(), config.block_on),
        ("none", vec!["critical".to_owned()])
    );

    // Switching on is the person's click on the confirmation, never an edit.
    assert!(edit_now(None, &store, &here, "enabled", json!(true)).is_err());

    write(
        dir.path(),
        "version: 1\nreview:\n  reviewers:\n    - agent: codex\n      persona: devflow-spec\n",
    );
    let outside = screen(None, &store, &here);
    assert_eq!(
        outside.config.expect("valid").reviewers,
        [DevflowReviewer {
            agent: "codex".into(),
            persona: "devflow-spec".into(),
            model: None,
        }]
    );
}

#[test]
fn a_value_past_the_ceiling_is_refused_before_it_is_read() {
    let big = format!("\"{}\"", "x".repeat(MOST_SENT));
    assert!(sent(&big, "the value").is_err());
    assert!(setup_params("custom", "[1]", false).is_err());
}

#[test]
fn a_new_setup_starts_where_settings_say() {
    let (_home, store) = store();
    assert_eq!(asks::preset(&store), Preset::Recommended);
    store
        .set_preference(preference::DEVFLOW_PRESET, "custom")
        .expect("kept");
    assert_eq!(prefs_of(&store).preset, "custom");
}
