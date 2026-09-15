use super::*;

fn lanes() -> Vec<String> {
    ["todo", "doing", "done"].map(String::from).to_vec()
}

#[test]
fn a_lane_with_cards_and_no_destination_is_refused_with_the_count() {
    assert_eq!(
        where_cards_go(&lanes(), "doing", None, 3).expect("answer"),
        Deleting::Refused(3)
    );
}

#[test]
fn a_destination_takes_the_cards() {
    assert_eq!(
        where_cards_go(&lanes(), "doing", Some("done"), 3).expect("answer"),
        Deleting::MovingTo("done".to_owned())
    );
}

#[test]
fn the_lane_itself_or_another_boards_lane_is_not_a_destination() {
    let itself = where_cards_go(&lanes(), "doing", Some("doing"), 3).expect_err("refused");
    assert_eq!(itself.code, ErrorCode::Invalid);
    let elsewhere =
        where_cards_go(&lanes(), "doing", Some("col_of_another_project"), 3).expect_err("refused");
    assert_eq!(elsewhere.code, ErrorCode::Invalid);
}

#[test]
fn an_empty_lane_still_takes_its_archived_cards_somewhere() {
    assert_eq!(
        where_cards_go(&lanes(), "doing", None, 0).expect("answer"),
        Deleting::MovingTo("todo".to_owned())
    );
    assert_eq!(
        where_cards_go(&["only".to_owned()], "only", None, 0).expect("answer"),
        Deleting::Plain
    );
}

#[test]
fn a_lane_of_another_board_is_not_found() {
    let refused = where_cards_go(&lanes(), "elsewhere", None, 0).expect_err("refused");
    assert_eq!(refused.code, ErrorCode::NotFound);
}
