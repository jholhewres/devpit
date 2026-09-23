use super::refuse_join;
use crate::sessions::tab_for_card;

#[test]
fn two_plain_tabs_join() {
    assert_eq!(refuse_join("tab_a", "tab_b"), None);
}

#[test]
fn a_tab_is_not_joined_into_itself() {
    assert!(refuse_join("tab_a", "tab_a").is_some());
}

#[test]
fn a_card_tab_is_neither_joined_away_nor_joined_into() {
    let card = tab_for_card("01CARD");
    assert!(refuse_join(&card, "tab_b").is_some());
    assert!(refuse_join("tab_a", &card).is_some());
}
