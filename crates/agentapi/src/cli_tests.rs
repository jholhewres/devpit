use super::*;

fn words(line: &str) -> Vec<String> {
    line.split_whitespace().map(ToOwned::to_owned).collect()
}

#[test]
fn nothing_or_help_is_the_guide() {
    assert_eq!(parsed(&[]), Ok(Parsed::Guide));
    assert_eq!(parsed(&words("--help")), Ok(Parsed::Guide));
}

#[test]
fn a_read_names_its_method() {
    assert_eq!(
        parsed(&words("board")),
        Ok(Parsed::Ask("board".into(), json!({})))
    );
    assert_eq!(
        parsed(&words("card crd_1")),
        Ok(Parsed::Ask("card".into(), json!({ "cardId": "crd_1" })))
    );
}

#[test]
fn a_comment_is_every_word_after_the_card() {
    assert_eq!(
        parsed(&words("comment crd_1 tests pass on main")),
        Ok(Parsed::Ask(
            "comment".into(),
            json!({ "cardId": "crd_1", "body": "tests pass on main" })
        ))
    );
    assert!(parsed(&words("comment crd_1")).is_err());
    assert_eq!(
        parsed(&words("comment-edit cmt_1 the right words")),
        Ok(Parsed::Ask(
            "comment_edit".into(),
            json!({ "commentId": "cmt_1", "body": "the right words" })
        ))
    );
    assert_eq!(
        parsed(&words("comment-delete cmt_1")),
        Ok(Parsed::Ask(
            "comment_delete".into(),
            json!({ "commentId": "cmt_1" })
        ))
    );
    assert!(parsed(&words("comment-edit cmt_1")).is_err());
}

#[test]
fn options_travel_under_the_names_the_app_reads() {
    assert_eq!(
        parsed(&words("create Fix login --body steps --column col_2")),
        Ok(Parsed::Ask(
            "create".into(),
            json!({ "title": "Fix login", "body": "steps", "columnId": "col_2" })
        ))
    );
    assert_eq!(
        parsed(&words("update crd_1 --title New")),
        Ok(Parsed::Ask(
            "update".into(),
            json!({ "cardId": "crd_1", "title": "New" })
        ))
    );
}

#[test]
fn an_unknown_command_is_refused_by_name() {
    assert_eq!(
        parsed(&words("deploy")),
        Err("`deploy` is not a devpit-agent command".to_owned())
    );
}

#[test]
fn devflows_session_start_hook_names_its_cli() {
    assert_eq!(
        parsed(&words("devflow-context --agent codex")),
        Ok(Parsed::Context("codex".into()))
    );
    assert_eq!(
        parsed(&words("devflow-context")),
        Ok(Parsed::Context("claude".into()))
    );
}

/// The guide names the command an agent can run: `devpit-agent` is what
/// devpit puts on its PATH, and `devpit` may be nothing there.
#[test]
fn the_guide_and_the_hint_name_the_command_on_the_agents_path() {
    assert!(PARSE_HINT.contains("devpit-agent guide"));
    assert!(!guide::GUIDE.contains("devpit agent "), "{}", guide::GUIDE);
    assert!(!guide::INSTRUCTIONS.contains("devpit agent "));
    for tool in &crate::mcp::TOOLS {
        assert!(!tool.description.contains("devpit agent "), "{}", tool.name);
    }
}

/// A stage on demand from a shell: its skill by name, or its tool with the
/// arguments the MCP tool takes.
#[test]
fn a_devflow_stage_is_its_skill_or_its_tool() {
    assert_eq!(
        parsed(&words("devflow review")),
        Ok(Parsed::Skill("devflow-review".into()))
    );
    let args: Vec<String> = ["devflow", "review", "--json", r#"{"pr": 42}"#]
        .iter()
        .map(|word| (*word).to_owned())
        .collect();
    assert_eq!(
        parsed(&args),
        Ok(Parsed::Ask("devflow_review".into(), json!({ "pr": 42 })))
    );
    assert!(parsed(&words("devflow deploy")).is_err());
    assert!(parsed(&words("devflow")).is_err());
    assert!(parsed(&words("devflow plan --json [1]")).is_err());
    assert!(parsed(&words("devflow plan --epic e1")).is_err());
    // Every stage's skill is one devpit ships, and its method a tool's.
    let tools = crate::devflow::STAGED
        .iter()
        .map(|(_, tool)| tool)
        .chain(&crate::devflow::SETUP_TOOLS);
    let methods: Vec<&str> = tools.map(|tool| tool.method).collect();
    for (stage, method) in crate::devflow::STAGE_METHODS {
        assert!(methods.contains(&method), "{method}");
        let Ok(Parsed::Skill(name)) = parsed(&words(&format!("devflow {stage}"))) else {
            panic!("{stage}");
        };
        assert!(crate::skills::NAMES.contains(&name.as_str()), "{name}");
    }
}
