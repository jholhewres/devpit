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
    first.cwd = Some("/w/card".to_owned());
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
    assert_eq!(later.cwd.as_deref(), Some("/w/card"));
    assert_eq!(later.cost_usd, 0.5);
    assert_eq!(later.budget_usd, Some(2.0));
    assert_eq!(later.permission.as_deref(), Some("plan"));
    assert_eq!(later.model.as_deref(), Some("haiku"));
    assert_eq!(later.effort.as_deref(), Some("high"));
}

#[test]
fn a_turn_runs_where_the_conversation_was_fixed_and_refuses_a_folder_that_is_gone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let here = dir.path().display().to_string();
    assert_eq!(turn_cwd(None, "/asked").expect("not fixed"), "/asked");
    assert_eq!(turn_cwd(Some(&here), "/asked").expect("fixed"), here);

    let gone = dir.path().join("gone").display().to_string();
    let refused = turn_cwd(Some(&gone), "/asked").expect_err("gone");
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert_eq!(
        refused.message,
        "the folder this conversation ran in is gone"
    );
}

#[test]
fn a_turn_ending_after_a_woken_one_keeps_what_that_one_wrote() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("conv.json");
    let began_with = opening(None, "claude", None, None, None, None, 1.0);
    // A woken turn ended while this one waited: its session and cost are on disk.
    let mut woken = began_with.clone();
    woken.session_id = Some("s_woken".to_owned());
    woken.cost_usd = 0.25;
    devpit_agentcli::head::write_head(&file, &woken).unwrap();

    let end = devpit_rpc::TurnEnd {
        turn_id: String::new(),
        cost_usd: Some(0.5),
        duration_ms: None,
        stop_reason: None,
        is_error: false,
        context: None,
    };
    settle(&file, Some(began_with), "turn_1", &end, None, None);

    let now = devpit_agentcli::head::read_head(&file).unwrap();
    assert_eq!(now.cost_usd, 0.75);
    assert_eq!(now.session_id.as_deref(), Some("s_woken"));
}
