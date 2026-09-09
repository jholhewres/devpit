use crate::head::{head_path, read_head, remaining, settled, write_head, Head};

fn head(profile: &str) -> Head {
    Head {
        profile: profile.to_owned(),
        model: None,
        card_id: None,
        created_at: 0.0,
        cost_usd: 0.0,
        budget_usd: None,
        session_id: None,
        permission: None,
        effort: None,
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
    let path = head_path(dir.path(), "proj", "conv");
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
    assert_eq!(read_head(&head_path(dir.path(), "proj", "missing")), None);
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
