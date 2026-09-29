use super::*;

#[test]
fn only_the_windows_no_tab_holds_are_strays() {
    let windows = vec!["leaf_held".to_owned(), "leaf_stray".to_owned()];
    let held: HashSet<String> = ["leaf_held".to_owned(), "leaf_not_made_yet".to_owned()].into();
    assert_eq!(unheld(&windows, &held), vec!["leaf_stray".to_owned()]);
}

#[test]
fn a_stray_window_goes_only_when_nothing_works_in_it() {
    assert!(
        closable(true, Some(0)),
        "an idle shell with nothing under it"
    );
    assert!(
        closable(true, None),
        "an idle shell where the table cannot be read"
    );
    assert!(!closable(true, Some(1)), "a job in the background");
    assert!(!closable(false, Some(0)), "an agent or a command in front");
    assert!(!closable(false, None));
}
