use super::*;
use serde_json::json;

fn project(id: &str, name: &str) -> Project {
    serde_json::from_value(json!({
        "id": id,
        "name": name,
        "rootPath": "/nowhere",
        "group": null,
        "accent": "#000000",
        "worktrees": [],
        "unreadable": null,
        "origin": null,
        "lastOpenedAt": null,
        "icon": null,
        "color": null,
    }))
    .expect("a project")
}

fn board(number: i64) -> Result<Option<String>, String> {
    Ok((number == 42).then(|| "card_01FORTYTWO".to_owned()))
}

#[test]
fn a_number_becomes_the_cards_id_under_every_key_that_holds_one() {
    let devpit = project("proj_1", "devpit");
    let params = json!({ "cardId": "#42", "card": "42", "epic": "devpit#42", "body": "#42" });
    let resolved = resolved(&params, &devpit, board).expect("resolved");
    assert_eq!(resolved["cardId"], "card_01FORTYTWO");
    assert_eq!(resolved["card"], "card_01FORTYTWO");
    assert_eq!(resolved["epic"], "card_01FORTYTWO");
    assert_eq!(resolved["body"], "#42", "only the keys that hold a card id");
}

#[test]
fn an_id_passes_through_untouched() {
    let devpit = project("proj_1", "devpit");
    let params = json!({ "cardId": "card_01M4VJA217154VNZPXH6CPF026" });
    let asked = |_| -> Result<Option<String>, String> { panic!("an id is not looked up") };
    assert_eq!(resolved(&params, &devpit, asked).expect("resolved"), params);
}

#[test]
fn a_number_no_card_holds_is_refused() {
    let devpit = project("proj_1", "devpit");
    let err = resolved(&json!({ "cardId": "#7" }), &devpit, board).expect_err("refused");
    assert!(err.contains("#7"), "{err}");
}

#[test]
fn a_number_of_another_project_is_not_looked_up_on_this_one() {
    let devpit = project("proj_1", "devpit");
    let err = resolved(&json!({ "cardId": "ascbot#42" }), &devpit, board).expect_err("refused");
    assert!(err.contains("another project"), "{err}");
}

#[test]
fn another_projects_name_picks_the_board_and_ones_own_does_not() {
    let here = project("proj_1", "devpit");
    let elsewhere = json!({ "cardId": "ascbot#3" });
    assert_eq!(project_named(&elsewhere, &here), Some("ascbot"));
    assert_eq!(project_named(&json!({ "cardId": "DevPit#3" }), &here), None);
    assert_eq!(project_named(&json!({ "cardId": "#3" }), &here), None);
}
