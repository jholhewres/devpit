use crate::head::{head_path, read_head, settled, write_head, Head};

fn head(profile: &str) -> Head {
    Head {
        profile: profile.to_owned(),
        model: None,
        card_id: None,
        created_at: 0.0,
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
        profile: "claudin".to_owned(),
        model: Some("opus".to_owned()),
        card_id: None,
        created_at: 12.0,
    };
    write_head(&path, &written).unwrap();
    assert_eq!(read_head(&path), Some(written));
}

#[test]
fn a_conversation_with_no_head_reads_as_none() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(read_head(&head_path(dir.path(), "proj", "missing")), None);
}
