use serde_json::json;

use super::*;

static STAGES: [(Option<&str>, Tool); 3] = [
    (
        None,
        Tool {
            name: "devpit_devflow_skill",
            method: "devflow_skill",
            description: "",
            input: || json!({ "type": "object" }),
        },
    ),
    (
        Some("plan"),
        Tool {
            name: "devpit_devflow_plan",
            method: "devflow_plan",
            description: "",
            input: || json!({ "type": "object" }),
        },
    ),
    (
        Some("review"),
        Tool {
            name: "devpit_devflow_review",
            method: "devflow_review",
            description: "",
            input: || json!({ "type": "object" }),
        },
    ),
];

fn names(offer: &Offer) -> Vec<&'static str> {
    offer.tools(&STAGES).iter().map(|tool| tool.name).collect()
}

#[test]
fn an_active_project_gets_the_tools_of_the_stages_switched_on() {
    assert!(names(&Offer::Off).is_empty());
    assert_eq!(
        names(&Offer::Setup),
        [
            "devpit_devflow_status",
            "devpit_devflow_setup",
            "devpit_devflow_set"
        ]
    );
    let active = names(&Offer::Active(vec!["briefing".into(), "plan".into()]));
    assert_eq!(
        &active[3..],
        ["devpit_devflow_skill", "devpit_devflow_plan"]
    );
}

#[test]
fn the_flow_joins_the_instructions_only_where_it_is_on() {
    assert_eq!(Offer::Off.instructions(), guide::INSTRUCTIONS);
    assert_eq!(Offer::Setup.instructions(), guide::INSTRUCTIONS);
    let on = Offer::Active(Vec::new()).instructions();
    assert!(on.starts_with(guide::INSTRUCTIONS) && on.ends_with(guide::DEVFLOW));
}

#[test]
fn the_apps_answer_is_read_as_an_offer() {
    assert_eq!(Offer::of(&json!({ "offer": "setup" })), Offer::Setup);
    assert_eq!(
        Offer::of(&json!({ "offer": "active", "stages": ["plan"] })),
        Offer::Active(vec!["plan".into()])
    );
    assert_eq!(Offer::of(&json!({ "offer": "off" })), Offer::Off);
    assert_eq!(Offer::of(&json!({})), Offer::Off);
}

#[test]
fn switching_devflow_on_or_off_tells_the_client_to_list_again() {
    let root = tempfile::tempdir().expect("root");
    let mut watch = Watch::new(root.path());
    let looked = watch.due().expect("the first look is due");
    assert_eq!(
        watch.settle(looked.clone(), Ok(json!({ "offer": "setup" }))),
        Some(LIST_CHANGED)
    );
    // Asked again with nothing new: nothing to say.
    assert_eq!(
        watch.settle(looked.clone(), Ok(json!({ "offer": "setup" }))),
        None
    );
    let on = json!({ "offer": "active", "stages": ["plan"] });
    assert_eq!(watch.settle(looked.clone(), Ok(on)), Some(LIST_CHANGED));
    assert_eq!(watch.offer, Offer::Active(vec!["plan".into()]));
    // A devpit that did not answer leaves the offer as it was.
    assert_eq!(watch.settle(looked.clone(), Err("closed".into())), None);
    assert_eq!(watch.offer, Offer::Active(vec!["plan".into()]));
    let off = json!({ "offer": "setup" });
    assert_eq!(watch.settle(looked, Ok(off)), Some(LIST_CHANGED));
    let sent: serde_json::Value = serde_json::from_str(LIST_CHANGED).expect("json");
    assert_eq!(sent["method"], "notifications/tools/list_changed");
    assert!(sent.get("id").is_none());
}

#[test]
fn the_watch_asks_again_only_when_a_file_it_watches_changed() {
    let root = tempfile::tempdir().expect("root");
    let file = root.path().join("devflow.yaml");
    let mut watch = Watch::new(root.path());
    let looked = watch.due().expect("due");
    let answer = json!({ "offer": "setup", "watch": file.display().to_string() });
    watch.settle(looked, Ok(answer));
    assert_eq!(watch.due(), None);

    std::fs::write(&file, "version: 1\nenabled: true\n").expect("write");
    let looked = watch.due().expect("the project's file changed");
    watch.settle(looked, Err("closed".into()));
    assert_eq!(watch.due(), None);

    std::fs::write(root.path().join(STAMP), "1\n").expect("stamp");
    assert!(watch.due().is_some(), "the app's stamp changed");
}
