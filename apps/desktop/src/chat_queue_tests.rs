use devpit_rpc::{Attachment, QueuedMessage};

use super::*;

fn queued(id: &str, files: Vec<Attachment>) -> QueuedMessage {
    QueuedMessage {
        id: id.to_owned(),
        prompt: format!("say {id}"),
        files,
        skills: Vec::new(),
    }
}

fn picture() -> Attachment {
    Attachment {
        name: "pasted-01.png".to_owned(),
        path: "/home/p/pasted/pasted-01.png".to_owned(),
        kind: "png".to_owned(),
    }
}

/// What a closed tab leaves behind is what the reopened one reads: the three
/// in their order, the picture on the message it was queued with.
#[test]
fn three_queued_messages_come_back_in_order_with_the_picture_on_its_own() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = queue_path(dir.path(), "conv_1");
    let kept = ChatQueue {
        items: vec![
            queued("q1", Vec::new()),
            queued("q2", vec![picture()]),
            queued("q3", Vec::new()),
        ],
        paused: false,
    };
    write_at(&path, &kept).expect("kept");

    let back = read_at(&path);
    let ids: Vec<&str> = back.items.iter().map(|item| item.id.as_str()).collect();
    assert_eq!(ids, ["q1", "q2", "q3"]);
    assert_eq!(back.items[1].files, vec![picture()]);
    assert!(back.items[0].files.is_empty() && back.items[2].files.is_empty());
    assert_eq!(back, kept);
}

#[test]
fn a_paused_queue_comes_back_paused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = queue_path(dir.path(), "conv_1");
    let kept = ChatQueue {
        items: vec![queued("q1", Vec::new())],
        paused: true,
    };
    write_at(&path, &kept).expect("kept");
    assert!(read_at(&path).paused);
}

#[test]
fn an_emptied_queue_leaves_no_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = queue_path(dir.path(), "conv_1");
    write_at(
        &path,
        &ChatQueue {
            items: vec![queued("q1", Vec::new())],
            paused: false,
        },
    )
    .expect("kept");
    write_at(&path, &ChatQueue::default()).expect("emptied");
    assert!(!path.exists());
    assert_eq!(read_at(&path), ChatQueue::default());
}

#[test]
fn a_file_past_the_ceiling_reads_as_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = queue_path(dir.path(), "conv_1");
    std::fs::create_dir_all(path.parent().expect("folder")).expect("folder");
    std::fs::write(&path, vec![b' '; MOST_BYTES as usize + 1]).expect("written");
    assert_eq!(read_at(&path), ChatQueue::default());
}

#[test]
fn a_queue_past_its_count_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = queue_path(dir.path(), "conv_1");
    let items = (0..=MOST_ITEMS)
        .map(|at| queued(&format!("q{at}"), Vec::new()))
        .collect();
    let refused = write_at(
        &path,
        &ChatQueue {
            items,
            paused: false,
        },
    )
    .expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Invalid);
    assert!(!path.exists());
}

/// The heads are every `.json` in the folder; a queue among them would be read as one.
#[test]
fn a_queue_is_not_among_the_heads() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = queue_path(dir.path(), "conv_1");
    assert_ne!(path.parent(), Some(dir.path()));
    assert!(path.starts_with(dir.path()));
}

#[test]
fn a_conversation_id_that_is_a_path_is_refused() {
    let refused = path_of("p1", "../conv").expect_err("refused");
    assert_eq!(refused.code, ErrorCode::Forbidden);
}
