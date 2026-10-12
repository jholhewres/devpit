use std::path::Path;

use super::*;

/// A project in the store, with the board it starts with.
fn project(dir: &Path) -> (Store, Project) {
    let store = crate::projects::store().expect("store");
    let id = store.add_project(dir, None).expect("project");
    store.ensure_board(&id).expect("board");
    let project = serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": dir.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": null,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project");
    (store, project)
}

fn cards() -> Value {
    json!([
        {
            "key": "s1", "title": "Cards are indexed by their body",
            "build": "A word in a card's body finds the card.",
            "criteria": ["a word in a body finds its card", "an empty query finds nothing"],
            "files": ["crates/core/src/store/search_index.rs"],
            "verify": "cargo test -p devpit-core search_index", "depends_on": []
        },
        { "key": "s2", "title": "The search box lists them", "criteria": ["typing lists"], "depends_on": ["s1"] },
        { "key": "s3", "title": "Matches are highlighted", "depends_on": ["s2"] }
    ])
}

fn titles(store: &Store, project: &Project) -> Vec<String> {
    let mut titles: Vec<String> = store
        .cards(&project.id)
        .expect("cards")
        .into_iter()
        .map(|card| card.title)
        .collect();
    titles.sort();
    titles
}

fn epic(store: &Store, project: &Project) -> String {
    let said = respond(
        None,
        "devflow_brief",
        project,
        &json!({
            "title": "Search by body", "goal": "Find a card by its body",
            "for_whom": "people with long boards", "success": "make test passes",
            "out_of_scope": ["comments"], "constraints": [], "decisions": ["one index"]
        }),
    )
    .expect("a brief");
    assert_eq!(said["created"], true);
    let id = said["cardId"].as_str().expect("an id").to_owned();
    assert_eq!(
        store.card(&id).expect("read").expect("card").title,
        "[devflow] Brief: Search by body"
    );
    id
}

#[test]
fn a_brief_becomes_an_epic_and_the_context_is_written_onto_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = project(dir.path());
    let epic = epic(&store, &project);
    let said = respond(
        None,
        "devflow_brief",
        &project,
        &json!({ "card": epic, "context": "## Context\n### Facts\n- titles are indexed (`a.rs:1`)" }),
    )
    .expect("the context");
    assert_eq!(said["created"], false);
    let body = store.card(&epic).expect("read").expect("card").body;
    assert!(
        body.starts_with("## Brief\n- Goal: Find a card by its body"),
        "{body}"
    );
    assert!(
        body.ends_with("## Context\n### Facts\n- titles are indexed (`a.rs:1`)"),
        "{body}"
    );
    assert_eq!(titles(&store, &project).len(), 1);

    let elsewhere = respond(
        None,
        "devflow_brief",
        &project,
        &json!({ "card": "card_nope", "goal": "x" }),
    );
    assert!(elsewhere.expect_err("no such card").contains("no card"));
}

#[test]
fn a_plan_is_a_preview_until_the_persons_yes_and_then_cards_that_wait_on_each_other() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = project(dir.path());
    let epic = epic(&store, &project);
    let plan = |cards: Value, apply: bool| {
        respond(
            None,
            "devflow_plan",
            &project,
            &json!({ "epic": epic, "cards": cards, "apply": apply }),
        )
        .expect("a plan")
    };

    let preview = plan(cards(), false);
    assert_eq!(preview["applied"], false);
    assert_eq!(
        preview["preview"][1]["title"],
        "[devflow] The search box lists them"
    );
    assert_eq!(preview["preview"][1]["blockedBy"], json!(["s1"]));
    assert_eq!(titles(&store, &project).len(), 1, "a preview created cards");
    assert_eq!(waiting(&store, &epic).expect("waiting").cards.len(), 3);

    // Cards the person has not seen are a new preview, not a yes.
    let mut changed = cards();
    changed[2]["title"] = json!("Matches are bold");
    assert_eq!(plan(changed, true)["applied"], false);
    assert_eq!(
        titles(&store, &project).len(),
        1,
        "unseen cards were created"
    );

    let mut seen = cards();
    seen[2]["title"] = json!("Matches are bold");
    let applied = plan(seen.clone(), true);
    assert_eq!(applied["applied"], true, "{applied}");
    assert_eq!(applied["approvedOnBoard"], false);
    let ids: Vec<String> = applied["cards"]
        .as_array()
        .expect("cards")
        .iter()
        .map(|card| card["cardId"].as_str().expect("id").to_owned())
        .collect();
    assert_eq!(applied["startsNow"], json!([ids[0]]));
    assert_eq!(
        titles(&store, &project),
        [
            "[devflow] Brief: Search by body",
            "[devflow] Cards are indexed by their body",
            "[devflow] Matches are bold",
            "[devflow] The search box lists them",
        ]
    );
    let edges = store.card_edges(&project.id).expect("edges");
    assert_eq!(
        edges.blockers,
        [
            (ids[1].clone(), ids[0].clone()),
            (ids[2].clone(), ids[1].clone())
        ]
    );
    assert!(edges.parents.iter().all(|(_, parent)| parent == &epic));
    let body = store.card(&ids[0]).expect("read").expect("card").body;
    assert_eq!(devpit_core::criteria::criteria(&body).len(), 2, "{body}");
    let said = store.comments(&epic).expect("comments");
    assert!(
        said[0]
            .body
            .starts_with("[devflow] Plan created after the person's yes"),
        "{}",
        said[0].body
    );

    // Applied twice, it is created once.
    assert_eq!(plan(seen, true)["cards"][0]["cardId"], json!(ids[0]));
    assert_eq!(titles(&store, &project).len(), 4);
}

#[test]
fn a_plan_the_person_edited_and_created_on_the_board_is_the_agents_answer() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = project(dir.path());
    let epic = epic(&store, &project);
    respond(
        None,
        "devflow_plan",
        &project,
        &json!({ "epic": epic, "cards": cards() }),
    )
    .expect("a preview");

    // The person cuts the last card and renames the second.
    let mut edited = waiting(&store, &epic).expect("waiting").cards;
    edited.pop();
    edited[1].title = "The search box lists matches".into();
    let kept = kept_on_board(&store, &project.id, &epic, edited).expect("kept");
    assert!(kept.edited);
    let made = created_on_board(&store, &project.id, &epic).expect("created");
    assert_eq!(made.card_ids.len(), 2);
    assert!(waiting(&store, &epic).is_none());

    let answer = respond(
        None,
        "devflow_plan",
        &project,
        &json!({ "epic": epic, "cards": cards(), "apply": true }),
    )
    .expect("the answer");
    assert_eq!(answer["approvedOnBoard"], true);
    assert_eq!(answer["cards"][1]["cardId"], json!(made.card_ids[1]));
    assert_eq!(
        answer["cards"][1]["title"],
        "[devflow] The search box lists matches"
    );
    assert_eq!(titles(&store, &project).len(), 3);
    assert_eq!(
        store.comments(&epic).expect("comments")[0].author,
        crate::cards::YOU
    );
}

#[test]
fn the_brief_and_the_plan_wait_for_the_persons_click_on_devflow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = project(dir.path());
    std::fs::create_dir_all(dir.path().join(".devpit/devflow")).expect("folder");
    std::fs::write(
        dir.path().join(".devpit/devflow/devflow.yaml"),
        "version: 1\nenabled: true\n",
    )
    .expect("yaml");
    for method in ["devflow_brief", "devflow_plan", "devflow_evidence"] {
        let refused = crate::devflow::respond(
            None,
            method,
            &project,
            dir.path(),
            &json!({ "title": "x", "card": "card_1" }),
        );
        assert!(refused.expect_err(method).contains("not on"), "{method}");
    }
    assert!(store.cards(&project.id).expect("cards").is_empty());
}
