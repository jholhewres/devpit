use devpit_devflow::Ask;
use devpit_rpc::Project;

use super::*;

fn project(id: &str, root: &Path) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": null,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

/// The MCP server cannot read flow/skills/, so it names the skills itself.
#[test]
fn the_skill_tool_offers_exactly_the_skills_devpit_ships() {
    assert_eq!(
        devpit_agentapi::skills::NAMES.to_vec(),
        skills::names().collect::<Vec<_>>()
    );
}

#[test]
fn an_active_project_gets_its_skills_its_own_layer_first() {
    let dir = tempfile::tempdir().expect("tempdir");
    let yaml = "version: 1\nenabled: true\nstages:\n  context: { enabled: false }\n";
    std::fs::create_dir_all(dir.path().join(".devpit/devflow")).expect("folder");
    std::fs::write(dir.path().join(".devpit/devflow/devflow.yaml"), yaml).expect("yaml");
    let here = project("prj_devflow_skills", dir.path());
    let ask = |method: &str, params: Value| crate::devflow::respond(None, method, &here, &params);

    // The file says enabled, but the person has not clicked: no skills.
    let refused = ask("devflow_skill", json!({ "name": "devflow-plan" })).expect_err("off");
    assert!(refused.contains("not on"), "{refused}");

    let store = crate::projects::store().expect("store");
    crate::devflow_asks::remember(&store, &here.id, Ask::Enable, true).expect("clicked");

    let listed = ask("devflow_skills", json!({})).expect("listed");
    let names: Vec<&str> = listed["skills"]
        .as_array()
        .expect("skills")
        .iter()
        .filter_map(|skill| skill["name"].as_str())
        .collect();
    assert_eq!(
        names,
        [
            "devflow-briefing",
            "devflow-plan",
            "devflow-execution",
            "devflow-review",
            "devflow-completion",
            "devflow-setup"
        ]
    );
    assert_eq!(listed["skills"][1]["layer"], "built-in");

    let ours = "---\nname: devflow-plan\ndescription: Our plan.\n---\n\nPlan our way.\n";
    let file = dir
        .path()
        .join(".devpit/devflow/skills/devflow-plan/SKILL.md");
    std::fs::create_dir_all(file.parent().expect("parent")).expect("folder");
    std::fs::write(&file, ours).expect("override");
    let plan = ask(
        "devflow_skill",
        json!({ "uri": "skill://devflow/devflow-plan/SKILL.md" }),
    )
    .expect("plan");
    assert_eq!(plan["layer"], "project");
    assert_eq!(plan["text"], ours);

    let off = ask("devflow_skill", json!({ "name": "devflow-context" })).expect_err("off");
    assert!(off.contains("switches off"), "{off}");
}
