use super::*;
use crate::mcp::handle_offering;

fn never(_: &str, _: Value) -> Result<Value, String> {
    panic!("the app should not have been asked")
}

/// The app, for an active project with the plan stage and the setup.
fn app(method: &str, params: Value) -> Result<Value, String> {
    let entry = |name: &str| {
        let uri = format!("skill://devflow/{name}/SKILL.md");
        json!({
            "name": name,
            "stage": if name == "devflow-setup" { Value::Null } else { json!("plan") },
            "layer": "built-in",
            "file": null,
            "skill": {
                "uri": uri,
                "frontmatter": { "name": name, "description": format!("Use for {name}.") },
                "resources": [{ "uri": uri, "digest": "sha256:ab", "size": 42 }],
            },
        })
    };
    match method {
        "devflow_skills" => Ok(json!({
            "skills": [entry("devflow-plan"), entry("devflow-setup")],
            "refused": [],
        })),
        "devflow_skill" => {
            let asked = params["name"]
                .as_str()
                .or_else(|| params["uri"].as_str()?.split('/').nth(3))
                .ok_or("no name")?;
            let mut said = entry(asked);
            said["text"] = json!(format!("---\nname: {asked}\n---\n\nDo {asked}.\n"));
            Ok(said)
        }
        _ => Err(format!("asked {method}")),
    }
}

fn answer(line: &str, offer: &Offer, ask: Ask) -> Value {
    serde_json::from_str(
        &handle_offering(line, ask, true, offer, &crate::listing::Seat::default())
            .expect("an answer"),
    )
    .expect("json")
}

fn active() -> Offer {
    Offer::Active(vec!["plan".into()])
}

#[test]
fn an_active_project_lists_its_skills_as_resources_with_their_entry() {
    let list = r#"{"jsonrpc":"2.0","id":1,"method":"resources/list"}"#;
    let listed = answer(list, &active(), &app);
    let resources = listed["result"]["resources"].as_array().expect("resources");
    let plan = resources
        .iter()
        .find(|one| one["name"] == "devflow-plan")
        .expect("the plan skill");
    assert_eq!(plan["uri"], "skill://devflow/devflow-plan/SKILL.md");
    assert_eq!(plan["mimeType"], "text/markdown");
    assert_eq!(plan["size"], 42);
    assert_eq!(
        plan["_meta"]["io.modelcontextprotocol.skills/skill"]["resources"][0]["digest"],
        "sha256:ab"
    );
    assert!(resources
        .iter()
        .any(|one| one["uri"] == "ui://devpit/board.html"));

    // Not on here: the pages alone, and the app is not asked.
    for offer in [Offer::Off, Offer::Setup] {
        let pages = answer(list, &offer, &never);
        assert!(pages["result"]["resources"]
            .as_array()
            .expect("resources")
            .iter()
            .all(|one| one["uri"].as_str().unwrap_or_default().starts_with("ui://")));
    }
}

#[test]
fn a_skill_is_read_as_a_resource_only_where_devflow_is_on() {
    let read = r#"{"jsonrpc":"2.0","id":2,"method":"resources/read","params":{"uri":"skill://devflow/devflow-plan/SKILL.md"}}"#;
    let said = answer(read, &active(), &app);
    let content = &said["result"]["contents"][0];
    assert_eq!(content["mimeType"], "text/markdown");
    assert_eq!(
        content["text"],
        "---\nname: devflow-plan\n---\n\nDo devflow-plan.\n"
    );

    let refused = answer(read, &Offer::Setup, &never);
    assert_eq!(refused["error"]["code"], -32002);
}

#[test]
fn each_stage_is_a_prompt_that_puts_its_skill_as_the_persons_message() {
    let list = r#"{"jsonrpc":"2.0","id":3,"method":"prompts/list"}"#;
    let listed = answer(list, &active(), &app);
    assert_eq!(
        listed["result"]["prompts"],
        json!([{ "name": "devflow-plan", "description": "Use for devflow-plan." }])
    );
    let get = r#"{"jsonrpc":"2.0","id":4,"method":"prompts/get","params":{"name":"devflow-plan"}}"#;
    let got = answer(get, &active(), &app);
    let message = &got["result"]["messages"][0];
    assert_eq!(message["role"], "user");
    assert_eq!(message["content"]["type"], "text");
    assert_eq!(
        message["content"]["text"],
        "---\nname: devflow-plan\n---\n\nDo devflow-plan.\n"
    );

    // The setup's skill is no stage, and nothing is a prompt where Devflow is off.
    let setup =
        r#"{"jsonrpc":"2.0","id":5,"method":"prompts/get","params":{"name":"devflow-setup"}}"#;
    assert_eq!(answer(setup, &active(), &app)["error"]["code"], -32602);
    assert_eq!(
        answer(list, &Offer::Setup, &never)["result"]["prompts"],
        json!([])
    );
    assert_eq!(answer(get, &Offer::Setup, &never)["error"]["code"], -32602);
}

#[test]
fn the_skill_tool_is_listed_in_an_active_project_and_says_the_skill_as_markdown() {
    let list = r#"{"jsonrpc":"2.0","id":6,"method":"tools/list"}"#;
    let tools = answer(list, &active(), &never);
    let tool = tools["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == TOOL)
        .cloned()
        .expect("listed");
    assert_eq!(
        tool["inputSchema"]["properties"]["name"]["enum"],
        json!(NAMES)
    );

    let call = r#"{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"devpit_devflow_skill","arguments":{"name":"devflow-plan"}}}"#;
    let said = answer(call, &active(), &app);
    assert_eq!(said["result"]["isError"], false);
    assert_eq!(
        said["result"]["content"][0]["text"],
        "---\nname: devflow-plan\n---\n\nDo devflow-plan.\n"
    );
    let about = said["result"]["content"][1]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(
        about.contains("built-in") && about.contains("sha256:ab"),
        "{about}"
    );

    // Asked where it is off, the app's refusal is the tool's error.
    let off = |_: &str, _: Value| -> Result<Value, String> { Err("Devflow is not on".into()) };
    assert_eq!(answer(call, &Offer::Setup, &off)["result"]["isError"], true);
}
