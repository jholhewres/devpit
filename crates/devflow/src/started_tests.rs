use super::*;

const CARD: &str = "card_01ABCDEF";

#[test]
fn the_first_start_is_kept_and_a_later_one_does_not_move_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(start_of(dir.path(), CARD), None);
    keep_start(dir.path(), CARD, "abc123").expect("kept");
    keep_start(dir.path(), CARD, "def456").expect("again");
    assert_eq!(start_of(dir.path(), CARD).as_deref(), Some("abc123"));
}

#[test]
fn a_start_that_is_not_a_commit_is_not_taken() {
    let dir = tempfile::tempdir().expect("tempdir");
    keep_start(dir.path(), CARD, "main; rm -rf /").expect("kept");
    assert_eq!(start_of(dir.path(), CARD), None);
}
