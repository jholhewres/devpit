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
        ended: None,
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
            created_at: 1.0,
            cost_usd: 0.5,
            budget_usd: Some(2.0),
            session_id: Some("s-orig".to_owned()),
            permission: None,
            effort: None,
            title: None,
            cwd: None,
            context: None,
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
            chat_rewind_now(project.into(), conversation.into(), turn.into()).expect_err("refused");
        assert_eq!(refused.code, ErrorCode::Forbidden);
    }
}

#[test]
fn editing_a_turn_forks_before_it_at_the_turn_before() {
    let home = tempfile::tempdir().expect("tempdir");
    two_turns(home.path());

    rewind_before(home.path(), "conv_a", "turn_2", "conv_b", 9.0).expect("edit");

    let (messages, _) = read(&conversation_path(home.path(), "conv_b"));
    assert_eq!(
        messages.len(),
        3,
        "turn 1, then the note: turn 2 goes back to the composer"
    );
    assert!(matches!(&messages[1].parts[0], Part::Text { text, .. } if text == "alpha"));
    let head = read_head(&head_path(home.path(), "conv_b")).expect("head");
    assert_eq!(head.rewind.fork_at.as_deref(), Some("u-1"));
    assert_eq!(head.session_id.as_deref(), Some("s-orig"));
}

#[test]
fn editing_the_first_turn_starts_the_cli_afresh() {
    let home = tempfile::tempdir().expect("tempdir");
    two_turns(home.path());

    rewind_before(home.path(), "conv_a", "turn_1", "conv_b", 9.0).expect("edit");

    let (messages, _) = read(&conversation_path(home.path(), "conv_b"));
    assert_eq!(
        messages.iter().map(|one| &one.parts[0]).collect::<Vec<_>>(),
        [&Part::Rewound {
            from_conversation: "conv_a".to_owned(),
            turn: 0
        }]
    );
    let head = read_head(&head_path(home.path(), "conv_b")).expect("head");
    assert_eq!((head.session_id, head.rewind.fork_at), (None, None));
    assert!(head.rewind.anchors.is_empty());
}

fn git(dir: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

/// Turn 2 edited a.txt and made b.txt; editing from it puts both back, and
/// leaves what turn 1 did.
#[test]
fn putting_back_undoes_the_turn_and_the_ones_after_it() {
    let home = tempfile::tempdir().expect("tempdir");
    two_turns(home.path());
    let checkout = tempfile::tempdir().expect("checkout");
    let at = checkout.path();
    git(at, &["init", "-q"]);
    std::fs::write(at.join("a.txt"), "turn 1\n").expect("write");
    git(at, &["add", "-A"]);
    let before = git(at, &["write-tree"]);
    std::fs::write(at.join("a.txt"), "turn 2\n").expect("edit");
    std::fs::write(at.join("b.txt"), "made\n").expect("make");
    let changes = Part::Changes {
        files: ["a.txt", "b.txt"]
            .map(|path| devpit_rpc::ChangedFile {
                path: path.to_owned(),
                added: 1,
                removed: 0,
            })
            .to_vec(),
        before: Some(before),
    };
    let answer = Message {
        parts: vec![changes],
        ..said("turn_2", Role::Assistant, "")
    };
    append(&conversation_path(home.path(), "conv_a"), &answer).expect("append");

    put_back_from(home.path(), at, "conv_a", "turn_2").expect("put back");

    assert_eq!(
        std::fs::read_to_string(at.join("a.txt")).expect("a"),
        "turn 1\n"
    );
    assert!(!at.join("b.txt").exists(), "made by the turn");
}

#[test]
fn a_turn_whose_files_were_not_kept_is_not_put_back() {
    let home = tempfile::tempdir().expect("tempdir");
    two_turns(home.path());
    let old = Message {
        parts: vec![Part::Changes {
            files: Vec::new(),
            before: None,
        }],
        ..said("turn_2", Role::Assistant, "")
    };
    append(&conversation_path(home.path(), "conv_a"), &old).expect("append");
    let refused = put_back_from(home.path(), home.path(), "conv_a", "turn_1").expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Conflict);
}
