//! The ring's tests, kept beside it.

use super::*;

#[test]
fn keeps_everything_while_it_fits() {
    let mut ring = RingBuffer::new(16);
    // Deliberately not a word from the project: this used to spell the
    // product name, and renaming it silently changed the byte count the
    // assertion below depends on.
    ring.write(b"abcdefgh");
    assert_eq!(ring.contents(), b"abcdefgh");
    assert_eq!(ring.len(), 8);
}

#[test]
fn drops_the_oldest_once_it_wraps() {
    let mut ring = RingBuffer::new(8);
    ring.write(b"abcdefgh");
    ring.write(b"XY");
    // The two oldest bytes are gone, and the order of the rest holds.
    assert_eq!(ring.contents(), b"cdefghXY");
    assert_eq!(ring.len(), 8);
}

#[test]
fn a_write_larger_than_the_buffer_leaves_its_tail() {
    let mut ring = RingBuffer::new(4);
    ring.write(b"abcdefghij");
    assert_eq!(ring.contents(), b"ghij");
}

/// The buffer against the obvious model of it: keep everything, then take
/// the last `cap` bytes.
///
/// The single cases above each pin one path. This walks a long sequence of
/// writes of every size around the capacity — under it, exactly it, over
/// it, and zero — and compares after each one, so a wrap that is right in
/// isolation and wrong in sequence has somewhere to fail.
///
/// The sequence is generated rather than typed, and seeded, so it covers
/// far more than anyone writes by hand and still fails the same way twice.
#[test]
fn it_holds_the_last_bytes_written_whatever_the_sizes() {
    for cap in [1usize, 2, 3, 4, 7, 8, 16] {
        let mut ring = RingBuffer::new(cap);
        let mut model: Vec<u8> = Vec::new();
        let mut seed = 0x2545_F491_4F6C_DD1Du64;
        let mut next = 0u8;

        for _ in 0..200 {
            // xorshift, so the sizes are spread rather than cycling.
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let len = (seed % (cap as u64 * 2 + 2)) as usize;

            let chunk: Vec<u8> = (0..len)
                .map(|_| {
                    next = next.wrapping_add(1);
                    next
                })
                .collect();

            ring.write(&chunk);
            model.extend_from_slice(&chunk);

            let kept = model.len().min(cap);
            let expected = &model[model.len() - kept..];
            assert_eq!(
                ring.contents(),
                expected,
                "cap {cap}, after writing {len} bytes"
            );
            assert_eq!(ring.len(), kept, "cap {cap}, len after {len} bytes");
            assert_eq!(ring.is_empty(), kept == 0, "cap {cap}");
        }
    }
}

/// What the replay is for: a wrap lands wherever it lands, and every
/// landing has to decode.
///
/// Walks every offset around the capacity rather than picking one, because
/// only some of them cut a character in half and a single case would pass
/// by luck.
#[test]
fn a_replay_always_starts_on_a_whole_character() {
    // Three bytes each, so two of every three wraps cut one in half.
    let text = "ção✓ção✓ção✓ção✓".repeat(4);
    for cap in 8usize..64 {
        let mut ring = RingBuffer::new(cap);
        ring.write(text.as_bytes());
        let replay = ring.replay();
        assert!(
            std::str::from_utf8(&replay).is_ok(),
            "cap {cap}: replay does not decode: {replay:?}"
        );
        // Only the partial character may go: dropping more would silently
        // eat output the person wrote.
        assert!(
            ring.contents().len() - replay.len() < 4,
            "cap {cap}: dropped {} bytes to find a boundary",
            ring.contents().len() - replay.len()
        );
    }
}

/// Nothing to trim is the common case, and it must not cost a byte.
#[test]
fn a_replay_that_already_starts_clean_is_untouched() {
    let mut ring = RingBuffer::new(16);
    ring.write("hello".as_bytes());
    assert_eq!(ring.replay(), b"hello");
}

#[test]
fn a_write_that_lands_exactly_on_the_end_does_not_lose_a_byte() {
    let mut ring = RingBuffer::new(4);
    ring.write(b"abcd");
    assert_eq!(ring.contents(), b"abcd");
    ring.write(b"e");
    assert_eq!(ring.contents(), b"bcde");
}
