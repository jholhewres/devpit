use super::*;

#[test]
fn a_frame_reads_back_as_written_and_two_in_a_row_stay_apart() {
    let one = Frame::new(7, Kind::Output, b"hello\x1b[0m".to_vec());
    let two = Frame::new(u32::MAX, Kind::Ack, 45_000u32.to_le_bytes().to_vec());
    let mut wire = one.encode();
    wire.extend(two.encode());
    let mut at = wire.as_slice();
    assert_eq!(read(&mut at).expect("one"), Some(one));
    assert_eq!(read(&mut at).expect("two"), Some(two));
    assert_eq!(read(&mut at).expect("end"), None);
}

#[test]
fn the_header_is_little_endian_length_pane_kind() {
    let wire = Frame::new(0x0102_0304, Kind::Input, vec![0xaa]).encode();
    assert_eq!(wire, [6, 0, 0, 0, 4, 3, 2, 1, Kind::Input as u8, 0xaa]);
}

#[test]
fn an_empty_body_is_a_frame() {
    let wire = Frame::new(1, Kind::Detach, Vec::new()).encode();
    let frame = read(&mut wire.as_slice()).expect("read").expect("frame");
    assert_eq!(frame.kind(), Some(Kind::Detach));
    assert!(frame.bytes.is_empty());
}

#[test]
fn a_length_out_of_step_is_refused_not_allocated() {
    for len in [0u32, 4, MOST_FRAME + 1, u32::MAX] {
        let mut wire = len.to_le_bytes().to_vec();
        wire.extend([0u8; 16]);
        let err = read(&mut wire.as_slice()).expect_err("refused");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{len}");
    }
}

#[test]
fn a_frame_cut_short_is_an_error_not_an_end() {
    let wire = Frame::new(1, Kind::Output, vec![1, 2, 3]).encode();
    let cut = &wire[..wire.len() - 1];
    assert!(read(&mut &cut[..]).is_err());
}

#[test]
fn an_unknown_kind_reads_and_is_named_none() {
    let wire = Frame {
        pane: 1,
        kind: 200,
        bytes: vec![],
    }
    .encode();
    let frame = read(&mut wire.as_slice()).expect("read").expect("frame");
    assert_eq!(frame.kind(), None);
    for byte in 1..=17u8 {
        assert_eq!(Kind::of(byte).map(|kind| kind as u8), Some(byte));
    }
}
