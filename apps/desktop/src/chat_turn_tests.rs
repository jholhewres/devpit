use super::*;

#[test]
fn a_first_turn_accepts_edits_and_starts_now() {
    let head = opening(None, "claude", None, None, None, None, 42.0);
    assert_eq!(head.permission.as_deref(), Some("acceptEdits"));
    assert_eq!(head.created_at, 42.0);
    assert_eq!(head.session_id, None);
}

#[test]
fn a_later_turn_keeps_what_the_conversation_settled() {
    let mut first = opening(
        None,
        "claude",
        None,
        Some(2.0),
        Some("plan".to_owned()),
        None,
        42.0,
    );
    first.session_id = Some("s1".to_owned());
    first.cost_usd = 0.5;

    let later = opening(
        Some(&first),
        "claude",
        Some("haiku".to_owned()),
        None,
        None,
        Some("high".to_owned()),
        99.0,
    );
    assert_eq!(later.created_at, 42.0);
    assert_eq!(later.session_id.as_deref(), Some("s1"));
    assert_eq!(later.cost_usd, 0.5);
    assert_eq!(later.budget_usd, Some(2.0));
    assert_eq!(later.permission.as_deref(), Some("plan"));
    assert_eq!(later.model.as_deref(), Some("haiku"));
    assert_eq!(later.effort.as_deref(), Some("high"));
}
