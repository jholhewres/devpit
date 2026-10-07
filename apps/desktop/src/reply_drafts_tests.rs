use super::{all, draft, drafted, forget, LONGEST};

#[test]
fn a_draft_is_kept_per_session_until_it_is_let_go() {
    draft("prof_t1", "api-a", "  yes, delete the folder  ").expect("drafted");
    draft("prof_t1", "api-b", "go on with F0b").expect("drafted");
    assert_eq!(
        drafted("prof_t1", "api-a").as_deref(),
        Some("yes, delete the folder")
    );
    assert_eq!(
        drafted("prof_t2", "api-a"),
        None,
        "another account's session"
    );

    draft("prof_t1", "api-a", "no, keep the rollback").expect("redrafted");
    assert_eq!(
        drafted("prof_t1", "api-a").as_deref(),
        Some("no, keep the rollback")
    );

    forget("prof_t1", "api-a");
    assert_eq!(drafted("prof_t1", "api-a"), None);
    assert_eq!(
        drafted("prof_t1", "api-b").as_deref(),
        Some("go on with F0b")
    );
}

#[test]
fn an_empty_or_overlong_draft_is_refused() {
    assert!(draft("prof_t3", "api-a", "   ").is_err());
    assert!(draft("prof_t3", "api-a", &"x".repeat(LONGEST + 1)).is_err());
    assert_eq!(drafted("prof_t3", "api-a"), None);
}

#[test]
fn every_draft_waiting_is_listed_for_the_remote() {
    draft("all-b", "web", "/compact").expect("drafted");
    draft("all-a", "api", "go on").expect("drafted");
    let listed: Vec<_> = all()
        .into_iter()
        .filter(|(profile, ..)| profile.starts_with("all-"))
        .collect();
    assert_eq!(
        listed,
        [
            ("all-a".to_owned(), "api".to_owned(), "go on".to_owned()),
            ("all-b".to_owned(), "web".to_owned(), "/compact".to_owned()),
        ]
    );
    forget("all-a", "api");
    forget("all-b", "web");
}

#[test]
fn a_sent_draft_is_recorded_with_where_it_went_from_and_whether_it_was_edited() {
    assert_eq!(
        super::sent_entry("api-a", "go on", " go on ", "the Remote"),
        "The person sent the draft for api-a from the Remote, as drafted: go on"
    );
    assert_eq!(
        super::sent_entry("api-a", "go on", "go on, but skip F2", "the window"),
        "The person sent the draft for api-a from the window, edited: go on, but skip F2"
    );
}
