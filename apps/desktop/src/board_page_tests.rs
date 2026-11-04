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

/// Moving a card into a lane renumbers it from 0: the reading still resumes
/// after the card it names, and the card that entered comes at the end.
#[test]
fn the_cursor_repeats_nothing_when_a_card_enters_the_lane_and_renumbers_it() {
    let mut board = board(30);
    let first = page(&board, &json!({ "column": "backlog", "limit": 10 }), row).expect("page 1");
    let mut seen = ids(&first["columns"][0]);
    let cursor = first["columns"][0]["nextCursor"]
        .as_str()
        .expect("a cursor")
        .to_owned();
    let mut entering = card("card_from_doing", "backlog", 0);
    entering.position = 0;
    board.cards.retain(|card| card.id != "card_doing_000");
    for card in board
        .cards
        .iter_mut()
        .filter(|card| card.column_id == "backlog")
    {
        card.position += 1;
    }
    board.cards.push(entering);
    let next = page(&board, &json!({ "cursor": cursor, "limit": 50 }), row).expect("page 2");
    seen.extend(ids(&next["columns"][0]));
    let mut unique = seen.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), seen.len(), "no card twice");
    assert_eq!(seen.len(), 31, "every card that was in the lane");
    assert_eq!(
        seen.last().map(String::as_str),
        Some("card_from_doing"),
        "the card that entered, at the end"
    );
}

#[test]
fn full_is_the_whole_board_whatever_came_with_it() {
    assert!(!asked(&json!({ "full": true, "column": "doing" })));
}

/// The cursor without its reading, as after the app restarted.
fn forgotten(cursor: &str) -> String {
    cursor.rsplit_once(':').expect("a reading").0.to_owned()
}

fn first_page(board: &Board, limit: usize) -> (Vec<String>, String) {
    let first = page(board, &json!({ "column": "backlog", "limit": limit }), row).expect("page 1");
    let cursor = first["columns"][0]["nextCursor"]
        .as_str()
        .expect("a cursor")
        .to_owned();
    (ids(&first["columns"][0]), cursor)
}

fn read_on(board: &Board, mut seen: Vec<String>, cursor: String) -> Vec<String> {
    let mut cursor = Some(cursor);
    while let Some(at) = cursor {
        let next = page(board, &json!({ "cursor": at, "limit": 7 }), row).expect("next page");
        seen.extend(ids(&next["columns"][0]));
        cursor = next["columns"][0]["nextCursor"].as_str().map(str::to_owned);
    }
    seen
}

/// Renumbers the lane from 0 in `order`, as move_card does.
fn renumber(board: &mut Board, order: &[String]) {
    for one in board
        .cards
        .iter_mut()
        .filter(|one| one.column_id == "backlog")
    {
        one.position = order
            .iter()
            .position(|id| *id == one.id)
            .expect("in the lane") as i32;
    }
}

fn backlog(of: usize) -> Vec<String> {
    (0..of).map(|at| format!("card_backlog_{at:03}")).collect()
}

fn moved(order: &mut Vec<String>, card: &str, to: usize) {
    order.retain(|id| id != card);
    order.insert(to, card.to_owned());
}

/// Read 10 of 30, move a card within the lane as move_card does (renumbering
/// it), read on.
fn moved_within(card: &str, to: usize, restart: bool) -> Vec<String> {
    let mut board = board(30);
    let (seen, cursor) = first_page(&board, 10);
    let mut order = backlog(30);
    moved(&mut order, card, to);
    renumber(&mut board, &order);
    let cursor = if restart { forgotten(&cursor) } else { cursor };
    read_on(&board, seen, cursor)
}

fn every_card_once(seen: &[String], of: usize) {
    for id in backlog(of) {
        let times = seen.iter().filter(|one| **one == id).count();
        assert_eq!(times, 1, "{id} was read {times} times");
    }
}

#[test]
fn a_card_moved_within_its_lane_between_pages_is_read_once_and_disturbs_no_other() {
    for (card, to) in [(9, 29), (10, 0), (9, 0), (10, 29), (25, 2), (3, 20)] {
        let seen = moved_within(&format!("card_backlog_{card:03}"), to, false);
        every_card_once(&seen, 30);
        assert_eq!(seen.len(), 30, "{card} to {to}");
    }
}

/// The finding of the last review: a lane with gaps in its places, one drag
/// renumbers it; then two drags in a dense lane.
#[test]
fn drags_in_a_lane_with_gaps_skip_no_card_that_never_moved() {
    let mut board = board(30);
    for one in board.cards.iter_mut() {
        one.position *= 10;
    }
    let (seen, cursor) = first_page(&board, 10);
    let mut order = backlog(30);
    moved(&mut order, "card_backlog_002", 15);
    renumber(&mut board, &order);
    let next = page(&board, &json!({ "cursor": cursor, "limit": 10 }), row).expect("page 2");
    let mut seen = seen;
    seen.extend(ids(&next["columns"][0]));
    let cursor = next["columns"][0]["nextCursor"]
        .as_str()
        .expect("a cursor")
        .to_owned();
    moved(&mut order, "card_backlog_028", 0);
    moved(&mut order, "card_backlog_012", 29);
    renumber(&mut board, &order);
    let seen = read_on(&board, seen, cursor);
    every_card_once(&seen, 30);
}

#[test]
fn a_card_that_leaves_the_lane_and_comes_back_unread_is_read_once() {
    let mut board = board(30);
    let (seen, cursor) = first_page(&board, 10);
    let away = board
        .cards
        .iter()
        .position(|card| card.id == "card_backlog_015")
        .expect("the card");
    board.cards[away].column_id = "doing".into();
    let next = page(&board, &json!({ "cursor": cursor, "limit": 10 }), row).expect("page 2");
    let mut seen = seen;
    seen.extend(ids(&next["columns"][0]));
    let cursor = next["columns"][0]["nextCursor"]
        .as_str()
        .expect("a cursor")
        .to_owned();
    board.cards[away].column_id = "backlog".into();
    let seen = read_on(&board, seen, cursor);
    every_card_once(&seen, 30);
}

/// The relay cuts a page shorter and names its own cursor in the same reading.
#[test]
fn a_page_cut_shorter_resumes_its_reading_after_the_last_card_kept() {
    let board = board(30);
    let first = page(&board, &json!({ "column": "backlog", "limit": 10 }), row).expect("page 1");
    let lane = &first["columns"][0];
    let reading = lane["reading"].as_str().expect("a reading");
    let kept: Vec<String> = ids(lane).into_iter().take(4).collect();
    let cursor = cursor("backlog", (3, &kept[3]), (4, "card_backlog_004"), reading);
    let seen = read_on(&board, kept, cursor);
    every_card_once(&seen, 30);
}

/// After a restart the cursor resumes by the places it names: only the card
/// that moved may repeat or be missed.
#[test]
fn a_cursor_whose_reading_is_gone_resumes_by_its_cards() {
    let seen = moved_within("card_backlog_009", 29, true);
    every_card_once_but(&seen, "card_backlog_009");
    let seen = moved_within("card_backlog_010", 0, true);
    every_card_once_but_maybe_missed(&seen, "card_backlog_010");
    let seen = moved_within("card_backlog_010", 29, true);
    every_card_once_but(&seen, "");
}

#[test]
fn a_crafted_cursor_with_extreme_places_is_answered_not_a_panic() {
    let cursor = format!("backlog:{}:card_x:{}:card_y", i64::MAX, i64::MIN);
    page(&board(5), &json!({ "cursor": cursor }), row).expect("a page");
    let cursor = format!(
        "backlog:{}:card_backlog_001:{}:card_backlog_003",
        i64::MIN,
        i64::MAX
    );
    page(&board(5), &json!({ "cursor": cursor }), row).expect("a page");
}

#[test]
fn a_reading_is_forgotten_once_it_expires_or_too_many_are_kept() {
    let now = Instant::now() + KEPT * 2;
    let reading = |at: Instant| Reading {
        at,
        order: Vec::new(),
        read: HashSet::new(),
        page: Vec::new(),
    };
    let mut readings = HashMap::new();
    readings.insert("old".to_owned(), reading(now - KEPT));
    readings.insert("new".to_owned(), reading(now));
    forget_old(&mut readings, now);
    assert_eq!(readings.keys().collect::<Vec<_>>(), ["new"]);
    for at in 0..READINGS {
        readings.insert(
            format!("r{at}"),
            reading(now + Duration::from_millis(at as u64 + 1)),
        );
    }
    forget_old(&mut readings, now);
    assert_eq!(readings.len(), READINGS - 1);
    assert!(!readings.contains_key("new"), "the oldest went");
}

fn every_card_once_but_maybe_missed(seen: &[String], moved: &str) {
    for at in 0..30 {
        let id = format!("card_backlog_{at:03}");
        let times = seen.iter().filter(|one| **one == id).count();
        assert!(
            times == 1 || (id == moved && times == 0),
            "{id} was read {times} times"
        );
    }
}

fn every_card_once_but(seen: &[String], moved: &str) {
    for at in 0..30 {
        let id = format!("card_backlog_{at:03}");
        let times = seen.iter().filter(|one| **one == id).count();
        assert!(times >= 1, "{id} was skipped");
        assert!(times == 1 || id == moved, "{id} was read {times} times");
    }
}
