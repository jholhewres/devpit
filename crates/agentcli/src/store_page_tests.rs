use devpit_rpc::{Message, Part, Role};

use super::{read_after, read_after_within, read_page, read_page_within, BLOCK};
use crate::store::{append, conversation_path, read};

fn message(id: &str, text: &str) -> Message {
    Message {
        id: id.to_owned(),
        turn_id: None,
        role: Role::Assistant,
        parts: vec![Part::Text {
            text: text.to_owned(),
            parent: None,
        }],
        created_at: 0.0,
        streaming: false,
    }
}

fn ids(messages: &[Message]) -> Vec<&str> {
    messages.iter().map(|one| one.id.as_str()).collect()
}

/// A conversation of `n` messages, some longer than a block.
fn written(n: usize) -> (tempfile::TempDir, std::path::PathBuf) {
    let home = tempfile::tempdir().expect("tempdir");
    let path = conversation_path(home.path(), "c1");
    for at in 0..n {
        let long = if at % 3 == 1 { 3 * BLOCK + 17 } else { 40 };
        append(&path, &message(&format!("m{at}"), &"é".repeat(long))).expect("append");
    }
    (home, path)
}

#[test]
fn the_last_page_is_the_newest_messages_oldest_first() {
    let (_home, path) = written(7);
    let page = read_page(&path, None, 3);
    assert_eq!(ids(&page.messages), ["m4", "m5", "m6"]);
    assert!(page.older);
    assert_eq!(page.skipped, 0);
}

#[test]
fn every_page_back_reads_the_whole_conversation_lines_longer_than_a_block_included() {
    let (_home, path) = written(10);
    let (whole, _) = read(&path);
    let mut pages = Vec::new();
    let mut before: Option<String> = None;
    loop {
        let page = read_page(&path, before.as_deref(), 2);
        before = page.messages.first().map(|one| one.id.clone());
        pages.splice(0..0, page.messages);
        if !page.older {
            break;
        }
    }
    assert_eq!(ids(&pages), ids(&whole));
    let json = |all: &[Message]| serde_json::to_string(all).expect("json");
    assert_eq!(
        json(&pages),
        json(&whole),
        "every byte of a line that spans blocks comes back"
    );
}

#[test]
fn a_page_of_one_from_two_messages_ends() {
    let (_home, path) = written(2);
    let page = read_page(&path, None, 1);
    assert_eq!(ids(&page.messages), ["m1"]);
    assert!(page.older);
    let rest = read_page(&path, Some("m1"), 1);
    assert_eq!(ids(&rest.messages), ["m0"]);
    assert!(!rest.older);
}

#[test]
fn a_bad_line_is_counted_and_a_missing_file_or_anchor_is_an_empty_page() {
    let (home, path) = written(3);
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, b"{ not json\n\n"))
        .expect("write");
    append(&path, &message("m3", "three")).expect("append");
    let page = read_page(&path, None, 10);
    assert_eq!(ids(&page.messages), ["m0", "m1", "m2", "m3"]);
    assert_eq!(page.skipped, 1);
    assert!(read_page(&path, Some("gone"), 10).messages.is_empty());
    assert!(
        read_page(&conversation_path(home.path(), "never"), None, 10)
            .messages
            .is_empty()
    );
}

#[test]
fn a_file_without_a_last_newline_still_gives_its_last_line() {
    let home = tempfile::tempdir().expect("tempdir");
    let path = conversation_path(home.path(), "c1");
    append(&path, &message("m0", "zero")).expect("append");
    let last = serde_json::to_string(&message("m1", &"x".repeat(BLOCK + 5))).expect("json");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, last.as_bytes()))
        .expect("write");
    assert_eq!(ids(&read_page(&path, None, 5).messages), ["m0", "m1"]);
}

/// The real transcript that froze the window: its last page, timed and weighed.
/// Local only: `PAGE_FIXTURE=<path to a .jsonl>`; the file is never versioned.
#[test]
#[ignore = "needs PAGE_FIXTURE, a transcript kept outside the repository"]
fn the_last_page_of_a_real_transcript_is_quick_and_light() {
    let path = std::path::PathBuf::from(std::env::var("PAGE_FIXTURE").expect("PAGE_FIXTURE"));
    let started = std::time::Instant::now();
    let mut page = read_page(&path, None, 50);
    page.messages
        .iter_mut()
        .for_each(devpit_rpc::cut::cut_message);
    let took = started.elapsed();
    let bytes = serde_json::to_string(&page.messages).expect("json").len();
    eprintln!(
        "{} messages, {bytes} bytes, read and cut in {took:?}, older: {}",
        page.messages.len(),
        page.older
    );
    assert_eq!(page.messages.len(), 50);
    assert!(bytes < 5_000_000, "{bytes} bytes");
}

#[test]
fn a_line_longer_than_the_ceiling_is_passed_over_and_counted() {
    let home = tempfile::tempdir().expect("tempdir");
    let path = conversation_path(home.path(), "c1");
    // The ceiling made small, so the line past it spans several blocks.
    let longest = 3 * BLOCK;
    append(&path, &message("m0", "zero")).expect("append");
    // A message, valid, past the ceiling, between two that are not:
    // kept, it would load; passed over, it is counted instead.
    append(&path, &message("big", &"z".repeat(longest + 10))).expect("append");
    append(&path, &message("m2", "two")).expect("append");
    let page = read_page_within(&path, None, 10, longest);
    assert_eq!(ids(&page.messages), ["m0", "m2"]);
    assert_eq!(page.skipped, 1);
    // Paging from the message after it still finds the one before.
    assert_eq!(
        ids(&read_page_within(&path, Some("m2"), 10, longest).messages),
        ["m0"]
    );
    // At the start of the file, the same.
    let start = conversation_path(home.path(), "c2");
    append(&start, &message("big", &"z".repeat(longest + 10))).expect("append");
    append(&start, &message("m1", "one")).expect("append");
    assert_eq!(
        ids(&read_page_within(&start, None, 10, longest).messages),
        ["m1"]
    );
}

#[test]
fn what_came_after_a_message_is_read_from_the_end_and_the_last_page_when_it_is_gone() {
    let (_home, path) = written(7);
    let (after, found) = read_after(&path, "m4", 3);
    assert!(found);
    assert_eq!(ids(&after), ["m5", "m6"]);
    let (last, found) = read_after(&path, "m6", 3);
    assert!(found);
    assert!(last.is_empty());
    let (page, found) = read_after(&path, "gone", 3);
    assert!(!found);
    assert_eq!(ids(&page), ["m4", "m5", "m6"]);
}

#[test]
fn what_came_after_skips_blank_bad_and_too_long_lines_and_reads_past_most() {
    let home = tempfile::tempdir().expect("tempdir");
    let path = conversation_path(home.path(), "c1");
    let longest = 2 * BLOCK;
    append(&path, &message("a", "anchor")).expect("append");
    std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .and_then(|mut file| std::io::Write::write_all(&mut file, b"\n  \n{ not json\n"))
        .expect("write");
    append(&path, &message("big", &"z".repeat(longest + 10))).expect("append");
    for at in 0..5 {
        append(&path, &message(&format!("m{at}"), "after")).expect("append");
    }
    // More after the anchor than `most`: all of them, since the anchor is there.
    let (after, found) = read_after_within(&path, "a", 2, longest);
    assert!(found);
    assert_eq!(ids(&after), ["m0", "m1", "m2", "m3", "m4"]);
    let (page, found) = read_after_within(&path, "gone", 2, longest);
    assert!(!found);
    assert_eq!(ids(&page), ["m3", "m4"]);
    let (none, found) = read_after(&conversation_path(home.path(), "never"), "a", 2);
    assert!(!found && none.is_empty());
}
