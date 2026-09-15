use devpit_agentcli::head::Anchor;
use devpit_rpc::Message;

use super::*;

fn said(turn: &str, role: Role, text: &str) -> Message {
    Message {
        id: format!("msg_{turn}_{text}"),
        turn_id: Some(turn.to_owned()),
        role,
        parts: vec![Part::Text {
            text: text.to_owned(),
            parent: None,
        }],
        created_at: 1.0,
        streaming: false,
    }
}

/// Two turns, both with their place in the CLI's transcript kept.
fn two_turns(home: &Path) {
    let file = conversation_path(home, "conv_a");
    for message in [
        said("turn_1", Role::User, "alpha?"),
        said("turn_1", Role::Assistant, "alpha"),
        said("turn_2", Role::User, "bravo?"),
        said("turn_2", Role::Assistant, "bravo"),
    ] {
        append(&file, &message).expect("append");
    }
    let anchor = |turn: &str, uuid: &str| Anchor {
        turn_id: turn.to_owned(),
        session_id: "s-orig".to_owned(),
        uuid: uuid.to_owned(),
    };
    write_head(
        &head_path(home, "conv_a"),
        &Head {
            profile: "claude".to_owned(),
            model: None,
            card_id: None,
            created_at: 1.0,
            cost_usd: 0.5,
            budget_usd: Some(2.0),
            session_id: Some("s-orig".to_owned()),
            permission: None,
            effort: None,
            title: None,
            rewind: Rewind {
                fork_at: None,
                anchors: vec![anchor("turn_1", "u-1"), anchor("turn_2", "u-2")],
            },
        },
    )
    .expect("head");
}

#[test]
fn a_rewind_forks_at_the_turn_and_leaves_the_original_alone() {
    let home = tempfile::tempdir().expect("tempdir");
    two_turns(home.path());
    let from = conversation_path(home.path(), "conv_a");
    let before = std::fs::read(&from).expect("read");

    rewind(home.path(), "conv_a", "turn_1", "conv_b", 9.0).expect("rewind");

    let (messages, _) = read(&conversation_path(home.path(), "conv_b"));
    assert_eq!(messages.len(), 3, "turn 1, then the note");
    assert!(matches!(&messages[1].parts[0], Part::Text { text, .. } if text == "alpha"));
    assert_eq!(
        messages[2].parts[0],
        Part::Rewound {
            from_conversation: "conv_a".to_owned(),
            turn: 1
        }
    );

    let head = read_head(&head_path(home.path(), "conv_b")).expect("head");
    assert_eq!(head.session_id.as_deref(), Some("s-orig"));
    assert_eq!(head.rewind.fork_at.as_deref(), Some("u-1"));
    assert_eq!(head.rewind.anchors.len(), 1, "only the turns it kept");
    assert_eq!((head.cost_usd, head.budget_usd), (0.5, Some(2.0)));

    assert_eq!(std::fs::read(&from).expect("read"), before);
}

#[test]
fn a_turn_with_no_kept_place_is_refused_rather_than_guessed() {
    let home = tempfile::tempdir().expect("tempdir");
    two_turns(home.path());
    let refused = rewind(home.path(), "conv_a", "turn_9", "conv_b", 9.0).expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Conflict);
    assert!(!conversation_path(home.path(), "conv_b").exists());
}

#[test]
fn ids_that_are_paths_are_refused() {
    for (project, conversation, turn) in
        [("../p", "c", "t"), ("p", "c/../d", "t"), ("p", "c", ".t")]
    {
        let refused =
            chat_rewind(project.into(), conversation.into(), turn.into()).expect_err("refused");
        assert_eq!(refused.code, ErrorCode::Forbidden);
    }
}
