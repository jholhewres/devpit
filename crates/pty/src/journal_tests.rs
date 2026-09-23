use super::*;

fn block(id: u64, output: &[u8]) -> Block {
    Block {
        head: Head {
            id,
            command: Some(format!("echo {id}")),
            cwd: Some("/work".to_owned()),
            started_at: 1000 + id,
            ended_at: Some(2000 + id),
            code: Some(i32::from(id == 2)),
            interactive: false,
            truncated: id == 3,
            bookmarked: false,
        },
        output: output.to_vec(),
    }
}

fn journal() -> (tempfile::TempDir, Journal) {
    let dir = tempfile::tempdir().expect("tempdir");
    let journal = Journal::at(dir.path().join("blocks").join("leaf.blocks"));
    (dir, journal)
}

fn ids(blocks: &[Block]) -> Vec<u64> {
    blocks.iter().map(|one| one.head.id).collect()
}

#[test]
fn a_block_comes_back_as_it_was_written() {
    let (_dir, mut journal) = journal();
    let mut odd = block(3, b"\x1b[31mred\x1b[0m \xff not utf-8\r\n");
    odd.head.command = None;
    journal.append(&block(1, b"one\r\n")).expect("append");
    journal.append(&odd).expect("append");
    let back = journal.load();
    assert_eq!(back.len(), 2);
    assert_eq!(back[0].head, block(1, b"").head);
    assert_eq!(back[1].head, odd.head);
    assert_eq!(back[1].output, odd.output, "raw bytes, not text");
}

#[test]
fn a_bookmark_is_kept_with_its_block() {
    let (_dir, mut journal) = journal();
    journal.append(&block(1, b"a")).unwrap();
    journal.append(&block(2, b"b")).unwrap();
    journal.mark(2, true).unwrap();
    journal.mark(1, true).unwrap();
    journal.mark(1, false).unwrap();
    let marked: Vec<bool> = journal
        .load()
        .iter()
        .map(|one| one.head.bookmarked)
        .collect();
    assert_eq!(marked, [false, true]);
}

#[test]
fn a_torn_last_record_costs_only_itself() {
    let (_dir, mut journal) = journal();
    journal.append(&block(1, b"whole")).unwrap();
    journal.append(&block(2, b"torn at the end")).unwrap();
    let bytes = std::fs::read(journal.path()).unwrap();
    std::fs::write(journal.path(), &bytes[..bytes.len() - 4]).unwrap();
    assert_eq!(ids(&journal.load()), [1]);
}

#[test]
fn a_length_past_its_ceiling_is_refused_before_it_is_read() {
    let mut bytes = MAGIC.to_vec();
    encode_block(&block(1, b"fine"), &mut bytes);
    encode_block(&block(2, b"x"), &mut bytes);
    // The second record's output length — the four bytes before its one byte
    // of output — rewritten to claim far more than a block may hold.
    let len_at = bytes.len() - 1 - 4;
    bytes[len_at..len_at + 4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(ids(&decode(&bytes)), [1]);
}

#[test]
fn a_file_that_is_not_a_journal_holds_nothing() {
    assert!(decode(b"something else entirely").is_empty());
}

#[test]
fn the_journal_keeps_only_the_newest_blocks() {
    let many: Vec<Block> = (1..=KEPT_BLOCKS as u64 + 5)
        .map(|id| block(id, b""))
        .collect();
    let few = kept(many);
    assert_eq!(few.len(), KEPT_BLOCKS);
    assert_eq!(few[0].head.id, 6, "the oldest go first");

    let big = vec![b'x'; KEPT_BYTES / 2 + 1];
    let heavy = kept(vec![block(1, &big), block(2, &big), block(3, &big)]);
    assert_eq!(ids(&heavy), [3], "and past the bytes too, oldest first");
}

#[test]
fn a_journal_past_twice_its_bound_is_rewritten_to_it() {
    let (_dir, mut journal) = journal();
    let last = 2 * KEPT_BLOCKS as u64 + 1;
    for id in 1..=last {
        journal.append(&block(id, b"line\r\n")).unwrap();
    }
    let size = std::fs::metadata(journal.path()).unwrap().len();
    let back = journal.load();
    assert_eq!(back.len(), KEPT_BLOCKS);
    assert_eq!(back.last().unwrap().head.id, last);
    // Rewritten, not merely read short: the file itself shrank.
    let mut one = Vec::new();
    encode_block(&block(last, b"line\r\n"), &mut one);
    assert!(size < (one.len() * (KEPT_BLOCKS + 2)) as u64, "{size}");
}

#[test]
fn removing_forgets_everything() {
    let (_dir, mut journal) = journal();
    journal.append(&block(1, b"a")).unwrap();
    journal.remove();
    assert!(!journal.path().exists());
    assert!(journal.load().is_empty());
}
