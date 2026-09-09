//! The scanner's tests, kept beside it.

use super::*;

/// Everything one chunk said.
fn heard(chunk: &[u8]) -> Vec<Told> {
    let mut scanner = Scanner::new();
    let mut told = Vec::new();
    scanner.scan(chunk, |one| told.push(one));
    told
}

/// The same input, handed over one byte at a time.
///
/// This is the shape the real stream has: frames are coalesced on a 16ms
/// boundary that knows nothing about escape sequences, so the cut lands
/// wherever it lands. A scanner that only works on whole sequences works
/// until the first busy terminal.
fn heard_in_pieces(chunk: &[u8], at: usize) -> Vec<Told> {
    let mut scanner = Scanner::new();
    let mut told = Vec::new();
    for piece in chunk.chunks(at) {
        scanner.scan(piece, |one| told.push(one));
    }
    told
}

#[test]
fn a_title_is_read_from_either_code() {
    assert_eq!(
        heard(b"\x1b]0;building\x07"),
        vec![Told::Title("building".to_owned())]
    );
    assert_eq!(
        heard(b"\x1b]2;building\x07"),
        vec![Told::Title("building".to_owned())]
    );
}

/// Both terminators are in the wild: `BEL` from most shells, `ESC \` from the
/// specification and from tmux.
#[test]
fn a_sequence_ends_at_either_terminator() {
    assert_eq!(
        heard(b"\x1b]2;done\x1b\\"),
        vec![Told::Title("done".to_owned())]
    );
}

#[test]
fn a_working_directory_comes_out_of_its_uri() {
    assert_eq!(
        heard(b"\x1b]7;file://box/home/jhol/devpit\x07"),
        vec![Told::Cwd("/home/jhol/devpit".to_owned())]
    );
}

/// A path with a space arrives encoded, and a directory nobody can open is
/// the same as no directory at all.
#[test]
fn a_percent_encoded_path_is_decoded() {
    assert_eq!(
        heard(b"\x1b]7;file:///home/jhol/my%20projects\x07"),
        vec![Told::Cwd("/home/jhol/my projects".to_owned())]
    );
}

/// The whole point of reading 133: a failure is a fact, not a guess at the
/// colour of some text.
#[test]
fn a_command_reports_the_code_it_ended_with() {
    assert_eq!(
        heard(b"\x1b]133;D;1\x07"),
        vec![Told::CommandEnded { code: Some(1) }]
    );
    assert_eq!(
        heard(b"\x1b]133;D;0\x07"),
        vec![Told::CommandEnded { code: Some(0) }]
    );
}

/// A missing code is not a zero. Reporting it as one paints a green mark for
/// a command nobody ran.
#[test]
fn a_command_that_reported_no_code_does_not_report_zero() {
    assert_eq!(
        heard(b"\x1b]133;D\x07"),
        vec![Told::CommandEnded { code: None }]
    );
}

#[test]
fn the_prompt_and_the_output_are_told_apart() {
    assert_eq!(
        heard(b"\x1b]133;A\x07\x1b]133;C\x07"),
        vec![Told::PromptBegan, Told::OutputBegan]
    );
}

#[test]
fn a_clipboard_write_carries_its_payload() {
    assert_eq!(
        heard(b"\x1b]52;c;SGVsbG8=\x07"),
        vec![Told::Clipboard("SGVsbG8=".to_owned())]
    );
}

/// Ordinary output is most of the stream and must pass through unremarked.
#[test]
fn plain_output_says_nothing() {
    assert!(heard(b"compiling devpit-pty v0.1.0\n").is_empty());
    // An escape that is not an OSC — this one is a colour — is not ours.
    assert!(heard(b"\x1b[31mred\x1b[0m").is_empty());
}

#[test]
fn a_code_this_build_has_no_opinion_about_is_ignored() {
    assert!(heard(b"\x1b]8;;https://example.invalid\x07").is_empty());
}

// ─── the boundary, which is where this earns its keep ───────────────

/// A sequence split anywhere still parses. Every offset, not one.
#[test]
fn a_sequence_cut_at_any_point_still_arrives() {
    let stream = b"out\x1b]7;file:///tmp/x\x07more\x1b]133;D;2\x07tail";
    let whole = heard(stream);
    assert_eq!(whole.len(), 2, "{whole:?}");

    for at in 1..stream.len() {
        assert_eq!(
            heard_in_pieces(stream, at),
            whole,
            "cut every {at} bytes lost something"
        );
    }
}

/// The introducer itself is two bytes, so it is the likeliest thing to be cut.
#[test]
fn an_introducer_split_across_two_chunks_is_still_one() {
    let mut scanner = Scanner::new();
    let mut told = Vec::new();
    scanner.scan(b"text\x1b", |one| told.push(one));
    scanner.scan(b"]2;after\x07", |one| told.push(one));
    assert_eq!(told, vec![Told::Title("after".to_owned())]);
}

/// The pre-filter must not swallow the case it exists to skip past.
#[test]
fn a_chunk_with_no_escape_leaves_nothing_behind() {
    let mut scanner = Scanner::new();
    scanner.scan(b"just output", |_| {
        panic!("said something about plain text")
    });
    scanner.scan(b"more output", |_| {
        panic!("said something about plain text")
    });

    // And the scanner still works afterwards, which is what proves the filter
    // did not leave it in a state that eats the next sequence.
    let mut told = Vec::new();
    scanner.scan(b"\x1b]2;still here\x07", |one| told.push(one));
    assert_eq!(told, vec![Told::Title("still here".to_owned())]);
}

/// The bytes come from a process that can write anything. An introducer that
/// is never terminated must not become a buffer that grows until the machine
/// stops.
#[test]
fn an_endless_sequence_is_dropped_rather_than_carried_forever() {
    let mut scanner = Scanner::new();
    let flood = vec![b'x'; MOST_CARRIED];
    scanner.scan(b"\x1b]2;", |_| {});
    for _ in 0..4 {
        scanner.scan(&flood, |_| {});
    }
    assert!(
        scanner.carried.len() <= MOST_CARRIED,
        "carried {} bytes",
        scanner.carried.len()
    );

    // And it recovers: the next real sequence is read.
    let mut told = Vec::new();
    scanner.scan(b"\x1b]2;back\x07", |one| told.push(one));
    assert_eq!(told, vec![Told::Title("back".to_owned())]);
}

/// A pane printing a build log says nothing about itself, and paying for a
/// walk of every byte of it is what the pre-filter exists to avoid. This pins
/// the behaviour the optimisation depends on rather than its speed.
#[test]
fn a_flood_with_no_introducer_carries_nothing_between_chunks() {
    let mut scanner = Scanner::new();
    scanner.scan(&vec![b'y'; 64 * 1024], |_| {});
    assert!(scanner.carried.is_empty());
}
