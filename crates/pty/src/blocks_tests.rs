use super::*;

fn tmux(body: &str) -> String {
    format!("\x1bPtmux;\x1b\x1b]{body}\x07\x1b\\")
}

fn run(chunks: &[&[u8]]) -> (Vec<Cut>, Vec<Block>) {
    let mut segmenter = Segmenter::new();
    let mut cuts = Vec::new();
    let mut ended = Vec::new();
    for (at, chunk) in chunks.iter().enumerate() {
        segmenter.feed(
            chunk,
            1000 * (at as u64 + 1),
            |_| {},
            |one, block| {
                cuts.push(one);
                ended.extend(block);
            },
        );
    }
    (cuts, ended)
}

#[test]
fn a_command_is_its_line_its_folder_its_output_and_its_code() {
    let stream = format!(
        "{}{}$ ls{}{}a.rs\r\nb.rs\r\n{}{}$ ",
        tmux("7;file://host/work"),
        tmux("133;A"),
        tmux("777;devpit-cmd;ls"),
        tmux("133;C"),
        tmux("133;D;0"),
        tmux("133;A"),
    );
    let (cuts, ended) = run(&[stream.as_bytes()]);
    assert_eq!(cuts.len(), 2);
    let block = &ended[0];
    assert_eq!(block.head.command.as_deref(), Some("ls"));
    assert_eq!(block.head.cwd.as_deref(), Some("/work"));
    assert_eq!(block.head.code, Some(0));
    // Not a byte of the envelopes, the prompt, or the marks.
    assert_eq!(block.output, b"a.rs\r\nb.rs\r\n");
}

#[test]
fn a_block_survives_being_split_across_reads_anywhere() {
    let stream = format!(
        "{}{}hello\r\n{}",
        tmux("777;devpit-cmd;echo hello"),
        tmux("133;C"),
        tmux("133;D;1")
    );
    let bytes = stream.as_bytes();
    for split in 1..bytes.len() {
        let (_, ended) = run(&[&bytes[..split], &bytes[split..]]);
        assert_eq!(ended.len(), 1, "split at {split}");
        assert_eq!(ended[0].output, b"hello\r\n", "split at {split}");
        assert_eq!(ended[0].head.code, Some(1));
    }
}

#[test]
fn a_prompt_with_a_command_still_open_ends_it_without_a_code() {
    let stream = format!(
        "{}{}working{}",
        tmux("777;devpit-cmd;sleep 9"),
        tmux("133;C"),
        tmux("133;A")
    );
    let (_, ended) = run(&[stream.as_bytes()]);
    assert_eq!(ended[0].head.code, None);
    assert_eq!(ended[0].output, b"working");
}

#[test]
fn a_repeated_end_or_a_line_announced_late_is_not_a_second_block() {
    // fish 4 marks its own C before devpit's line; and says D once per mark set.
    let stream = format!(
        "\x1b]133;C\x1b\\{}out{}{}",
        tmux("777;devpit-cmd;make"),
        tmux("133;D;0"),
        tmux("133;D;0")
    );
    let (cuts, ended) = run(&[stream.as_bytes()]);
    assert_eq!(ended.len(), 1);
    assert_eq!(ended[0].head.command.as_deref(), Some("make"));
    assert_eq!(
        cuts.iter()
            .filter(|one| matches!(one, Cut::Ended(_)))
            .count(),
        1
    );
}

#[test]
fn a_full_screen_program_is_marked_and_a_flood_keeps_its_end() {
    let mut flood =
        format!("{}{}\x1b[?1049h", tmux("777;devpit-cmd;vim"), tmux("133;C")).into_bytes();
    flood.extend(std::iter::repeat_n(b'x', MOST_OUTPUT + 10));
    flood.extend(b"END");
    flood.extend(tmux("133;D;0").as_bytes());
    let (_, ended) = run(&[&flood]);
    let block = &ended[0];
    assert!(block.head.interactive);
    assert!(block.head.truncated);
    assert_eq!(block.output.len(), MOST_OUTPUT);
    assert!(block.output.ends_with(b"END"));
}

#[test]
fn history_forgets_the_oldest_first() {
    let mut history = History::default();
    for id in 0..(MOST_BLOCKS as u64 + 5) {
        history.push(Block {
            head: Head {
                id,
                command: None,
                cwd: None,
                started_at: 0,
                ended_at: None,
                code: None,
                interactive: false,
                truncated: false,
            },
            output: Vec::new(),
        });
    }
    assert_eq!(history.heads().len(), MOST_BLOCKS);
    assert!(history.get(0).is_none());
    assert!(history.get(MOST_BLOCKS as u64 + 4).is_some());
}

/// A block as the test compares it: line, folder, output, code.
type Seen<'a> = (Option<&'a str>, Option<&'a str>, &'a [u8], Option<i32>);

#[test]
fn two_commands_cut_into_three_reads_anywhere_come_out_the_same() {
    let stream = format!(
        "{}{}$ {}{}one\r\n{}{}{}$ {}{}two{}{}",
        tmux("7;file://h/a"),
        tmux("133;A"),
        tmux("777;devpit-cmd;echo one"),
        tmux("133;C"),
        tmux("133;D;0"),
        tmux("7;file://h/b"),
        tmux("133;A"),
        tmux("777;devpit-cmd;printf two"),
        tmux("133;C"),
        tmux("133;D;3"),
        tmux("133;A"),
    );
    let bytes = stream.as_bytes();
    for first in 1..bytes.len() {
        for second in (first + 1)..bytes.len() {
            let (_, ended) = run(&[&bytes[..first], &bytes[first..second], &bytes[second..]]);
            let seen: Vec<Seen> = ended
                .iter()
                .map(|one| {
                    (
                        one.head.command.as_deref(),
                        one.head.cwd.as_deref(),
                        &one.output[..],
                        one.head.code,
                    )
                })
                .collect();
            assert_eq!(
                seen,
                [
                    (Some("echo one"), Some("/a"), &b"one\r\n"[..], Some(0)),
                    (Some("printf two"), Some("/b"), &b"two"[..], Some(3)),
                ],
                "cut at {first} and {second}"
            );
        }
    }
}
