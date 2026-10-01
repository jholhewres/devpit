use super::{draft, drafted, forget, LONGEST};

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
