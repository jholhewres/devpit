use super::*;

#[test]
fn a_tail_never_starts_inside_an_escape_sequence() {
    let mut history = History::new(1024);
    history.write(b"\x1b[38;5;196mred line\r\n\x1b[0mplain");
    // A cut that lands in the middle of the first SGR moves to the next ESC
    // or line start, whichever is first.
    // 25 bytes back lands inside `\x1b[38;5;196m`'s line: the next ESC wins.
    assert_eq!(history.tail(25), b"\x1b[0mplain");
}

#[test]
fn a_tail_of_text_starts_after_a_newline() {
    let mut history = History::new(1024);
    history.write(b"first line\nsecond line\nthird");
    // 14 bytes back lands inside "second line": the tail starts at the next line.
    assert_eq!(history.tail(14), b"third");
}

#[test]
fn a_tail_without_esc_or_newline_starts_on_a_whole_character() {
    let mut history = History::new(64);
    history.write("ação ".repeat(4).as_bytes());
    let tail = history.tail(9);
    assert!(std::str::from_utf8(&tail).is_ok(), "{tail:?}");
}

#[test]
fn a_tail_bigger_than_the_history_is_all_of_it() {
    let mut history = History::new(1024);
    history.write(b"\x1b[?1049hall");
    assert_eq!(history.tail(REPLAYED), b"\x1b[?1049hall");
}

#[test]
fn older_pages_walk_back_to_the_start_and_join_up() {
    let mut history = History::new(4 * PAGE);
    let mut written = Vec::new();
    for n in 0..30_000 {
        let line = format!("\x1b[32mline {n}\x1b[0m\r\n");
        written.extend_from_slice(line.as_bytes());
        history.write(line.as_bytes());
    }
    let mut got = history.tail(REPLAYED);
    loop {
        let (page, left) = history.older(got.len());
        let mut joined = page;
        joined.extend_from_slice(&got);
        got = joined;
        if left == 0 {
            break;
        }
    }
    let kept = &written[written.len() - history.len()..];
    assert_eq!(got, kept, "every page together is the whole history");
}

#[test]
fn the_history_keeps_its_last_two_megabytes() {
    let mut history = History::new(2 * 1024 * 1024);
    let chunk = vec![b'x'; 45 * 1024];
    for _ in 0..100 {
        history.write(&chunk);
    }
    assert_eq!(history.len(), 2 * 1024 * 1024);
}

/// A history that never filled its ring is the whole stream: replayed whole,
/// first line included, whatever it starts with.
#[test]
fn a_short_history_is_replayed_whole() {
    let mut history = History::new(1024);
    history.write(b"first line\nsecond\n");
    assert_eq!(history.tail(REPLAYED), b"first line\nsecond\n");
    let mut no_newline = History::new(1024);
    no_newline.write(b"$ prompt \x1b[1mbold");
    assert_eq!(no_newline.tail(REPLAYED), b"$ prompt \x1b[1mbold");
}

/// A full ring starts wherever the oldest overwrite left it: that start is cut.
#[test]
fn a_full_ring_is_cut_at_its_oldest_byte() {
    let mut history = History::new(16);
    history.write(b"\x1b[31mabcdefgh\nrest-of-it");
    let tail = history.tail(REPLAYED);
    assert_eq!(tail, b"rest-of-it");
}
