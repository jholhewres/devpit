use devflow_engine::Ask;
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
    assert_eq!(
        devpit_agentapi::skills::PARTS.to_vec(),
        PARTS.map(|(name, part, _)| (name, part)).to_vec()
    );
}

#[test]
fn an_active_project_gets_its_skills_its_own_layer_first() {
    let dir = tempfile::tempdir().expect("tempdir");
    let yaml = "version: 1\nenabled: true\nstages:\n  context: { enabled: false }\n";
    std::fs::create_dir_all(dir.path().join(".devpit/devflow")).expect("folder");
    std::fs::write(dir.path().join(".devpit/devflow/devflow.yaml"), yaml).expect("yaml");
    let here = project("prj_devflow_skills", dir.path());
    // The store is shared with earlier runs, which leave this switch-on behind.
    let store = crate::projects::store().expect("store");
    crate::devflow_asks::remember(&store, &here.id, Ask::Enable, false).expect("forgot");
    let ask = |method: &str, params: Value| {
        crate::devflow::respond_from(None, method, &here, dir.path(), None, &params)
    };

    // The file says enabled, but the person has not clicked: none listed,
    // and one asked by name is still given, to run its stage on demand.
    let refused = ask("devflow_skills", json!({})).expect_err("off");
    assert!(refused.contains("not on"), "{refused}");
    let asked = ask("devflow_skill", json!({ "name": "devflow-review" })).expect("on demand");
    assert_eq!(asked["layer"], "built-in");

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

    // A stage devflow.yaml switches off is not listed, but given when asked.
    assert!(!names.contains(&"devflow-context"));
    let off = ask("devflow_skill", json!({ "name": "devflow-context" })).expect("asked for");
    assert_eq!(off["name"], "devflow-context");
}

/// The review skill's size before its on-demand parts left it.
const REVIEW_WHOLE: usize = 14_694;

/// Each part of the review skill a card's session does not need, by name,
/// with the heading it opens with.
const REVIEW_PARTS: [(&str, &str); 5] = [
    ("verify", "# Devflow review: verify the criticals and highs"),
    ("without-card", "# Devflow review: without a card"),
    (
        "retro",
        "# Devflow review: retro, code already in production",
    ),
    ("publish", "# Devflow review: publish on the pull request"),
    (
        "by-hand",
        "# Devflow review: by hand, without devpit's tool",
    ),
];

/// A project whose devflow.yaml says enabled, in a fresh folder.
fn enabled_in(id: &str) -> (tempfile::TempDir, Project) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".devpit/devflow")).expect("folder");
    std::fs::write(
        dir.path().join(".devpit/devflow/devflow.yaml"),
        "version: 1\nenabled: true\n",
    )
    .expect("yaml");
    let here = project(id, dir.path());
    (dir, here)
}

#[test]
fn the_review_skill_served_per_card_is_a_third_shorter_and_its_parts_are_served_on_demand() {
    let (dir, here) = enabled_in("prj_devflow_review_parts");
    let ask = |method: &str, params: Value| {
        crate::devflow::respond_from(None, method, &here, dir.path(), None, &params)
    };

    let core = ask("devflow_skill", json!({ "name": "devflow-review" })).expect("the skill");
    let text = core["text"].as_str().expect("text");
    assert!(
        text.len() * 3 <= REVIEW_WHOLE * 2,
        "{} chars, more than two thirds of {REVIEW_WHOLE}",
        text.len()
    );
    let listed: Vec<&str> = core["skill"]["resources"]
        .as_array()
        .expect("resources")
        .iter()
        .filter_map(|one| one["uri"].as_str())
        .collect();

    for (part, heading) in REVIEW_PARTS {
        assert!(
            text.contains(&format!("\"part\": \"{part}\"")),
            "the skill points to {part}"
        );
        let uri = format!("skill://devflow/devflow-review/{part}.md");
        assert!(listed.contains(&uri.as_str()), "{uri} in {listed:?}");
        let by_name = ask(
            "devflow_skill",
            json!({ "name": "devflow-review", "part": part }),
        )
        .expect("by name");
        assert_eq!(by_name["part"], part);
        assert_eq!(by_name["layer"], "built-in");
        assert!(
            by_name["text"].as_str().expect("text").starts_with(heading),
            "{part}"
        );
        let by_uri = ask("devflow_skill", json!({ "uri": uri })).expect("by uri");
        assert_eq!(by_uri["text"], by_name["text"]);
    }

    let unknown = ask(
        "devflow_skill",
        json!({ "name": "devflow-review", "part": "nope" }),
    )
    .expect_err("not a part");
    assert!(unknown.contains("without-card"), "{unknown}");
    let elsewhere = ask(
        "devflow_skill",
        json!({ "name": "devflow-plan", "part": "retro" }),
    )
    .expect_err("the plan has no parts");
    assert!(elsewhere.contains("devflow-plan"), "{elsewhere}");
}

/// A project that writes its own review skill keeps its own parts beside
/// it; a part it did not write is devpit's.
#[test]
fn a_projects_own_review_skill_serves_its_own_parts_first() {
    let (dir, here) = enabled_in("prj_devflow_review_own_parts");
    let folder = dir.path().join(".devpit/devflow/skills/devflow-review");
    std::fs::create_dir_all(&folder).expect("folder");
    let ours = "---\nname: devflow-review\ndescription: Our review.\n---\n\nReview our way.\n";
    std::fs::write(folder.join("SKILL.md"), ours).expect("skill");
    std::fs::write(folder.join("retro.md"), "Our retro.\n").expect("part");
    let ask = |params: Value| {
        crate::devflow::respond_from(None, "devflow_skill", &here, dir.path(), None, &params)
    };

    let retro = ask(json!({ "name": "devflow-review", "part": "retro" })).expect("ours");
    assert_eq!(retro["layer"], "project");
    assert_eq!(retro["text"], "Our retro.\n");
    let publish = ask(json!({ "name": "devflow-review", "part": "publish" })).expect("devpit's");
    assert_eq!(publish["layer"], "built-in");
    assert!(publish["text"]
        .as_str()
        .expect("text")
        .starts_with(REVIEW_PARTS[3].1));
}
