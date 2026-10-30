use super::*;

#[test]
fn a_vim_in_the_alternate_screen_with_mouse_is_put_back_in_both() {
    let mut modes = Modes::new();
    modes.feed(b"\x1b[?1049h\x1b[?1h\x1b=\x1b[?1000;1006hdrawn\x1b[?25l");
    assert_eq!(modes.restore(), b"\x1b[?1;1049;1000;1006h\x1b[?25l");
}

#[test]
fn a_mode_turned_off_again_is_not_put_back() {
    let mut modes = Modes::new();
    modes.feed(b"\x1b[?1049h\x1b[?2004h\x1b[?25l");
    modes.feed(b"\x1b[?1049l\x1b[?25h");
    assert_eq!(modes.restore(), b"\x1b[?2004h");
}

#[test]
fn a_sequence_split_across_chunks_is_still_heard() {
    let mut modes = Modes::new();
    let whole = b"text\x1b[?1049;2004h more";
    for cut in 1..whole.len() {
        let mut split = Modes::new();
        split.feed(&whole[..cut]);
        split.feed(&whole[cut..]);
        assert_eq!(split.restore(), b"\x1b[?1049;2004h", "cut at {cut}");
    }
    modes.feed(whole);
    assert_eq!(modes.restore(), b"\x1b[?1049;2004h");
}

#[test]
fn other_sequences_and_unknown_modes_change_nothing() {
    let mut modes = Modes::new();
    modes.feed(b"\x1b[31mred\x1b[0m\x1b]7;file:///tmp\x07\x1b[?12h\x1b[?7727h\x1b[2J");
    assert!(modes.restore().is_empty());
}

#[test]
fn a_run_that_never_ends_is_not_kept_forever() {
    let mut modes = Modes::new();
    let mut runaway = b"\x1b[?".to_vec();
    runaway.extend(std::iter::repeat_n(b'1', 200));
    modes.feed(&runaway);
    modes.feed(b"h\x1b[?2004h");
    assert_eq!(modes.restore(), b"\x1b[?2004h");
}
