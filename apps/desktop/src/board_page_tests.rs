use super::*;

fn column(id: &str, role: Option<&str>) -> Column {
    serde_json::from_value(json!({
        "id": id, "name": id, "position": 0, "step": null, "onPass": null, "autonomy": "manual", "role": role,
    }))
    .expect("a column")
}

fn card(id: &str, column: &str, position: i32) -> Card {
    serde_json::from_value(json!({
        "id": id, "columnId": column, "title": format!("The card called {id}"), "body": "", "position": position,
        "worktreePath": null, "dueAt": null, "dueTime": false, "costUsd": 0.0, "comments": 0, "pinned": 0,
        "runs": [], "activity": null,
    }))
    .expect("a card")
}

/// `per` cards in each of a backlog, a doing and a done lane.
fn board(per: usize) -> Board {
    let lanes = [
        ("backlog", None),
        ("doing", Some("doing")),
        ("ship", Some("done")),
    ];
    Board {
        project_id: "p".into(),
        columns: lanes.iter().map(|(id, role)| column(id, *role)).collect(),
        cards: lanes
            .iter()
            .flat_map(|(lane, _)| {
                (0..per).map(move |at| card(&format!("card_{lane}_{at:03}"), lane, at as i32))
            })
            .collect(),
        steps: Vec::new(),
    }
}

fn row(card: &Card) -> Value {
    crate::agent_api::card_row(card)
}

fn ids(lane: &Value) -> Vec<String> {
    lane["cards"]
        .as_array()
        .map(|cards| {
            cards
                .iter()
                .map(|card| card["id"].as_str().unwrap_or_default().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_board_of_300_cards_answers_a_page_well_under_20k_characters() {
    let answer = page(&board(100), &json!({ "limit": LIMIT }), row).expect("a page");
    let text = answer.to_string();
    assert!(text.len() < 20_000, "{} characters", text.len());
    let lanes = answer["columns"].as_array().expect("lanes");
    assert_eq!(lanes[0]["total"], 100);
    assert_eq!(ids(&lanes[0]).len(), LIMIT);
    assert!(lanes[0]["nextCursor"].is_string());
    // The finished lane is counted, not listed.
    assert_eq!(lanes[2]["total"], 100);
    assert!(lanes[2].get("cards").is_none());
}

#[test]
fn only_a_call_that_says_how_asks_for_a_page() {
    assert!(!asked(&json!({})));
    assert!(!asked(&json!({ "project": "p", "cardId": null })));
    assert!(asked(&json!({ "limit": 5 })));
    assert!(asked(&json!({ "done": true })));
}

/// Read a lane page by page; a card moved to another lane between pages.
#[test]
fn the_cursor_reads_every_card_once_even_when_one_moves_between_pages() {
    let mut board = board(45);
    let first = page(&board, &json!({ "column": "backlog", "limit": 10 }), row).expect("page 1");
    let mut seen = ids(&first["columns"][0]);
    let mut cursor = first["columns"][0]["nextCursor"]
        .as_str()
        .map(str::to_owned);
    // A card already read leaves the lane: a count would now skip one.
    board.cards.retain(|card| card.id != "card_backlog_003");
    while let Some(at) = cursor {
        let next = page(&board, &json!({ "cursor": at, "limit": 10 }), row).expect("next page");
        seen.extend(ids(&next["columns"][0]));
        cursor = next["columns"][0]["nextCursor"].as_str().map(str::to_owned);
    }
    let mut unique = seen.clone();
    unique.dedup();
    assert_eq!(unique.len(), seen.len(), "no card twice");
    assert_eq!(seen.len(), 45, "no card skipped");
}

#[test]
fn a_finished_lane_lists_its_cards_when_asked() {
    let answer = page(&board(3), &json!({ "done": true }), row).expect("a page");
    assert_eq!(ids(&answer["columns"][2]).len(), 3);
    let named = page(&board(3), &json!({ "column": "ship" }), row).expect("a page");
    assert_eq!(ids(&named["columns"][0]).len(), 3);
}

#[test]
fn a_page_filters_by_text_and_epic() {
    let mut board = board(5);
    board.cards[1].parent_id = Some("card_backlog_000".into());
    let epic = page(&board, &json!({ "epic": "card_backlog_000" }), row).expect("a page");
    assert_eq!(
        ids(&epic["columns"][0]),
        ["card_backlog_000", "card_backlog_001"]
    );
    let text = page(&board, &json!({ "text": "BACKLOG_004" }), row).expect("a page");
    assert_eq!(text["columns"][0]["total"], 1);
}

#[test]
fn a_cursor_the_board_did_not_give_is_refused_and_says_so() {
    let why = page(&board(1), &json!({ "cursor": "nonsense" }), row).expect_err("refused");
    assert!(why.contains("nextCursor"));
}
