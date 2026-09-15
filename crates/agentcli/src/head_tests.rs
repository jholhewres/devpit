use crate::head::{head_path, read_head, remaining, settled, write_head, Head};

fn head(profile: &str) -> Head {
    Head {
        profile: profile.to_owned(),
        model: None,
        created_at: 0.0,
        cost_usd: 0.0,
        budget_usd: None,
        session_id: None,
        permission: None,
        effort: None,
        title: None,
        rewind: Default::default(),
        cwd: None,
    }
}

#[test]
fn a_new_conversation_takes_the_profile_it_is_given() {
    assert_eq!(settled(None, "claudin"), Ok(()));
}

#[test]
fn the_same_profile_runs() {
    assert_eq!(settled(Some(&head("claude")), "claude"), Ok(()));
}

#[test]
fn another_account_is_refused_by_name() {
    assert_eq!(settled(Some(&head("claude")), "claudin"), Err("claude"));
}

#[test]
fn a_head_survives_the_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = head_path(dir.path(), "conv");
    let written = Head {
        model: Some("opus".to_owned()),
        created_at: 12.0,
        cost_usd: 0.4,
        session_id: Some("sess_1".to_owned()),
        ..head("claudin")
    };
    write_head(&path, &written).unwrap();
    assert_eq!(read_head(&path), Some(written));
}

#[test]
fn a_conversation_with_no_head_reads_as_none() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_head(&head_path(dir.path(), "missing")), None);
}

#[test]
fn a_conversation_with_no_cap_has_nothing_left_to_check() {
    assert_eq!(remaining(Some(&head("claude"))), None);
}

#[test]
fn a_cap_counts_down_by_what_was_spent() {
    let spent = Head {
        budget_usd: Some(2.0),
        cost_usd: 0.75,
        ..head("claude")
    };
    assert_eq!(remaining(Some(&spent)), Some(1.25));
}

#[test]
fn a_spent_conversation_has_nothing_left() {
    let spent = Head {
        budget_usd: Some(1.0),
        cost_usd: 1.0,
        ..head("claude")
    };
    assert_eq!(remaining(Some(&spent)), Some(0.0));
}

fn opened(fork_at: Option<&str>) -> crate::head::Head {
    crate::head::Head {
        profile: "claude".to_owned(),
        model: None,
        created_at: 1.0,
        cost_usd: 1.0,
        budget_usd: None,
        session_id: Some("orig".to_owned()),
        permission: None,
        effort: None,
        title: None,
        rewind: crate::head::Rewind {
            fork_at: fork_at.map(str::to_owned),
            anchors: Vec::new(),
        },
        cwd: None,
    }
}

#[test]
fn a_turn_that_wrote_a_message_is_a_place_to_rewind_to() {
    let head = crate::head::after_turn(
        opened(None),
        "turn_1",
        Some(0.5),
        Some("s1".to_owned()),
        Some("u1".to_owned()),
    );
    assert_eq!(head.rewind.anchors.len(), 1);
    let anchor = &head.rewind.anchors[0];
    assert_eq!(
        (
            anchor.turn_id.as_str(),
            anchor.session_id.as_str(),
            anchor.uuid.as_str()
        ),
        ("turn_1", "s1", "u1")
    );
    assert_eq!(
        (head.session_id.as_deref(), head.cost_usd),
        (Some("s1"), 1.5)
    );
}

#[test]
fn a_fork_is_settled_by_the_first_message_the_forked_turn_writes() {
    let head = crate::head::after_turn(
        opened(Some("u0")),
        "turn_2",
        None,
        Some("s-new".to_owned()),
        Some("u9".to_owned()),
    );
    assert_eq!(head.rewind.fork_at, None);
    assert_eq!(head.session_id.as_deref(), Some("s-new"));
}

/// The CLI printed a new session id but the turn died before writing anything:
/// the next turn has to fork from the original again, not resume that id.
#[test]
fn a_forked_turn_that_wrote_nothing_forks_again_next_time() {
    let head = crate::head::after_turn(
        opened(Some("u0")),
        "turn_2",
        None,
        Some("s-new".to_owned()),
        None,
    );
    assert_eq!(head.rewind.fork_at.as_deref(), Some("u0"));
    assert_eq!(head.session_id.as_deref(), Some("orig"));
}

/// A conversation begun on a card has no account until its first turn picks one.
#[test]
fn a_conversation_opened_without_a_profile_takes_the_first_turns() {
    assert_eq!(settled(Some(&head("")), "prof_glm"), Ok(()));
    assert_eq!(settled(Some(&head("prof_a")), "prof_glm"), Err("prof_a"));
}
