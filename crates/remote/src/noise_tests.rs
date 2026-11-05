use super::{Error, Handshake, Keys, Opened, Session, MOST_CHUNK, MOST_FRAME};

const HOST: &str = "host_01NOISETEST";

/// A device and a machine, through a whole handshake.
fn sessions(most: usize) -> (Session, Session, Keys, Keys) {
    let (phone, machine) = (Keys::generate().unwrap(), Keys::generate().unwrap());
    let mut device = Handshake::initiator(&phone, HOST).unwrap();
    let mut host = Handshake::responder(&machine, HOST).unwrap();
    host.read(&device.write().unwrap()).unwrap();
    device.read(&host.write().unwrap()).unwrap();
    host.read(&device.write().unwrap()).unwrap();
    assert!(device.finished() && host.finished());
    (
        device.into_session(most).unwrap(),
        host.into_session(most).unwrap(),
        phone,
        machine,
    )
}

fn carried(from: &mut Session, to: &mut Session, message: &[u8]) -> Result<Vec<u8>, Error> {
    let mut whole = None;
    for frame in from.seal(message)? {
        assert!(frame.len() <= MOST_FRAME);
        whole = to.open(&frame)?;
    }
    whole.ok_or(Error::Broken)
}

#[test]
fn a_handshake_proves_both_keys_and_opens_a_session_each_way() {
    let (mut device, mut host, phone, machine) = sessions(1 << 20);
    assert_eq!(device.peer(), machine.public);
    assert_eq!(host.peer(), phone.public);
    assert_eq!(
        carried(&mut device, &mut host, b"{\"t\":\"projects\"}").unwrap(),
        b"{\"t\":\"projects\"}"
    );
    assert_eq!(carried(&mut host, &mut device, b"").unwrap(), b"");
}

#[test]
fn a_message_longer_than_a_frame_crosses_in_frames_and_comes_out_whole() {
    let (mut device, mut host, ..) = sessions(1 << 20);
    let board = vec![b'x'; MOST_CHUNK * 3 + 7];
    let frames = host.seal(&board).unwrap();
    assert_eq!(frames.len(), 4);
    let mut whole = None;
    for (at, frame) in frames.iter().enumerate() {
        whole = device.open(frame).unwrap();
        assert_eq!(whole.is_some(), at == 3, "whole only at its last frame");
    }
    assert_eq!(whole.unwrap(), board);
}

#[test]
fn a_frame_changed_dropped_or_repeated_on_the_way_does_not_open() {
    let (mut device, mut host, ..) = sessions(1 << 20);
    let mut changed = device.seal(b"type ls").unwrap().remove(0);
    changed[3] ^= 0x40;
    assert!(matches!(host.open(&changed), Err(Error::Broken)));

    let (mut device, mut host, ..) = sessions(1 << 20);
    let _dropped = device.seal(b"one").unwrap();
    let second = device.seal(b"two").unwrap().remove(0);
    assert!(
        matches!(host.open(&second), Err(Error::Broken)),
        "the first was dropped"
    );

    let (mut device, mut host, ..) = sessions(1 << 20);
    let one = device.seal(b"one").unwrap().remove(0);
    assert_eq!(host.open(&one).unwrap().as_deref(), Some(&b"one"[..]));
    assert!(
        matches!(host.open(&one), Err(Error::Broken)),
        "a frame said twice"
    );
}

#[test]
fn a_message_past_the_ceiling_is_refused_before_it_is_kept() {
    let (mut device, mut host, ..) = sessions(100);
    assert_eq!(
        carried(&mut device, &mut host, &[b'a'; 100]).unwrap().len(),
        100
    );
    assert!(matches!(
        carried(&mut device, &mut host, &[b'a'; 101]),
        Err(Error::TooBig)
    ));

    // Across frames too: the ceiling counts what is held, not one frame.
    let (mut device, mut host, ..) = sessions(MOST_CHUNK + 10);
    let frames = device.seal(&vec![b'a'; MOST_CHUNK * 2]).unwrap();
    assert_eq!(host.open(&frames[0]).unwrap(), None);
    assert!(matches!(host.open(&frames[1]), Err(Error::TooBig)));
    assert!(matches!(
        host.open(&vec![0u8; MOST_FRAME + 1]),
        Err(Error::TooBig)
    ));
}

#[test]
fn a_handshake_meant_for_another_machine_does_not_finish() {
    let (phone, machine) = (Keys::generate().unwrap(), Keys::generate().unwrap());
    let mut device = Handshake::initiator(&phone, "host_01OTHER").unwrap();
    let mut host = Handshake::responder(&machine, HOST).unwrap();
    host.read(&device.write().unwrap()).unwrap();
    assert!(matches!(
        device.read(&host.write().unwrap()),
        Err(Error::Broken)
    ));
}

#[test]
fn speaking_out_of_turn_is_refused() {
    let keys = Keys::generate().unwrap();
    let mut host = Handshake::responder(&keys, HOST).unwrap();
    assert!(matches!(host.write(), Err(Error::OutOfTurn)));
    let mut device = Handshake::initiator(&keys, HOST).unwrap();
    assert!(matches!(device.read(&[0u8; 32]), Err(Error::OutOfTurn)));
    assert!(matches!(device.into_session(10), Err(Error::OutOfTurn)));
}

#[test]
fn a_key_never_shows_its_secret_half() {
    let keys = Keys::generate().unwrap();
    let shown = format!("{keys:?}");
    assert!(shown.contains(&super::hex(&keys.public)));
    assert!(!shown.contains(&super::hex(&keys.secret)));
}

#[test]
fn raw_bytes_cross_as_bytes_and_json_as_json_in_one_session() {
    let (mut host, mut device, _, _) = sessions(1 << 20);
    let output = vec![0x1b, b'[', b'H', 0xff, 0x00];
    for frame in host.seal_bytes(&output).unwrap() {
        if let Some(whole) = device.open_any(&frame).unwrap() {
            assert_eq!(whole, Opened::Bytes(output.clone()));
        }
    }
    let said = host.seal(br#"{"t":"pong"}"#).unwrap().remove(0);
    assert_eq!(
        device.open_any(&said).unwrap(),
        Some(Opened::Text(br#"{"t":"pong"}"#.to_vec()))
    );
}

#[test]
fn raw_bytes_to_an_end_that_reads_only_json_break_the_session() {
    let (mut host, mut device, _, _) = sessions(1 << 20);
    let frame = host.seal_bytes(b"ls\r").unwrap().remove(0);
    assert!(matches!(device.open(&frame), Err(Error::Broken)));
}

/// One frame with `flag`, as no sealing of a whole message makes it.
fn frame(from: &mut Session, flag: u8, chunk: &[u8]) -> Vec<u8> {
    let mut plain = vec![flag];
    plain.extend_from_slice(chunk);
    let mut out = vec![0u8; plain.len() + 16];
    let len = from.transport.write_message(&plain, &mut out).unwrap();
    out.truncate(len);
    out
}

#[test]
fn a_message_that_changes_kind_halfway_does_not_open() {
    let (mut host, mut device, _, _) = sessions(1 << 20);
    let first = frame(&mut host, super::MORE, b"{");
    let then = frame(&mut host, super::BYTES_LAST, b"x");
    assert_eq!(device.open_any(&first).unwrap(), None);
    assert!(matches!(device.open_any(&then), Err(Error::Broken)));
}
