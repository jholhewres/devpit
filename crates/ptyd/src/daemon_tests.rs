use std::io;
use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use super::*;
use crate::pane::Asked;

/// A daemon listening on a socket of its own, admitting `own`.
fn listening(own: u32) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("dir");
    let at = dir.path().join("devpit-ptyd.sock");
    let listener = socket::listen(&at).expect("listen");
    let daemon = Daemon::new();
    std::thread::spawn(move || daemon.serve_as(listener, own));
    (dir, at)
}

fn connected(at: &std::path::Path) -> UnixStream {
    let mut stream = UnixStream::connect(at).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .expect("timeout");
    assert_eq!(
        socket::hello(&mut stream, socket::PROTOCOL).expect("hello"),
        socket::PROTOCOL
    );
    stream
}

fn send(stream: &mut UnixStream, pane: u32, kind: Kind, bytes: impl Into<Vec<u8>>) {
    frame::write(stream, &Frame::new(pane, kind, bytes)).expect("send");
}

/// Frames read until `until` holds, or the stream stays quiet.
fn read_until(stream: &mut UnixStream, until: impl Fn(&[Frame]) -> bool) -> Vec<Frame> {
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut got = Vec::new();
    while Instant::now() < deadline && !until(&got) {
        match frame::read(stream) {
            Ok(Some(one)) => got.push(one),
            _ => break,
        }
    }
    got
}

fn printed(frames: &[Frame]) -> Vec<u8> {
    frames
        .iter()
        .filter(|f| matches!(f.kind(), Some(Kind::Output | Kind::Replay)))
        .flat_map(|f| f.bytes.clone())
        .collect()
}

fn shell() -> Asked {
    Asked {
        rows: 24,
        cols: 80,
        cwd: std::env::temp_dir().to_string_lossy().into_owned(),
        argv: vec!["/bin/sh".into()],
    }
}

/// Over the real socket: spawn, attach, type, see it run; then a second
/// connection, as a desktop that restarted, is shown what happened.
#[test]
fn a_pane_typed_into_over_the_socket_is_replayed_to_the_next_connection() {
    let (_dir, at) = listening(socket::own_uid());
    let mut first = connected(&at);
    send(&mut first, 9, Kind::Spawn, shell().encode());
    send(&mut first, 9, Kind::Attach, Vec::new());
    send(
        &mut first,
        9,
        Kind::Input,
        b"echo ptyd-$((40+2))\n".to_vec(),
    );
    let seen = read_until(&mut first, |got| {
        printed(got).windows(9).any(|w| w == b"ptyd-42\r\n")
    });
    assert!(
        printed(&seen).windows(9).any(|w| w == b"ptyd-42\r\n"),
        "{}",
        String::from_utf8_lossy(&printed(&seen))
    );
    drop(first);

    let mut second = connected(&at);
    send(&mut second, 9, Kind::Attach, Vec::new());
    let replayed = read_until(&mut second, |got| {
        printed(got).windows(7).any(|w| w == b"ptyd-42")
    });
    assert!(
        replayed.iter().any(|f| f.kind() == Some(Kind::Replay)),
        "a late connection is sent the replay"
    );
    send(&mut second, 9, Kind::Kill, Vec::new());
    let ended = read_until(&mut second, |got| {
        got.iter().any(|f| f.kind() == Some(Kind::Exited))
    });
    assert!(ended.iter().any(|f| f.kind() == Some(Kind::Exited)));
}

#[test]
fn a_request_for_a_pane_that_is_not_there_is_refused_and_the_connection_lives() {
    let (_dir, at) = listening(socket::own_uid());
    let mut stream = connected(&at);
    send(&mut stream, 404, Kind::Input, b"x".to_vec());
    let got = read_until(&mut stream, |got| !got.is_empty());
    assert_eq!(got[0].kind(), Some(Kind::Refused));
    send(&mut stream, 1, Kind::Spawn, vec![0, 0]);
    let got = read_until(&mut stream, |got| !got.is_empty());
    assert_eq!(got[0].kind(), Some(Kind::Refused), "a bad spawn is refused");
}

/// A peer of another uid is closed before it is greeted: the hello gets no
/// answer. The daemon is told its own uid is someone else's, which is how a
/// test stands where another user's process would.
#[test]
fn a_connection_from_another_user_is_closed_unanswered() {
    let (_dir, at) = listening(socket::own_uid().wrapping_add(1));
    let mut stream = UnixStream::connect(&at).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("timeout");
    let err = socket::hello(&mut stream, socket::PROTOCOL).expect_err("not answered");
    assert!(
        matches!(
            err.kind(),
            io::ErrorKind::InvalidData
                | io::ErrorKind::UnexpectedEof
                | io::ErrorKind::BrokenPipe
                | io::ErrorKind::ConnectionReset
        ),
        "{err:?}"
    );
}

#[test]
fn a_client_of_another_protocol_is_told_the_daemons_and_served_nothing() {
    let (_dir, at) = listening(socket::own_uid());
    let mut stream = UnixStream::connect(&at).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("timeout");
    assert_eq!(
        socket::hello(&mut stream, socket::PROTOCOL + 7).expect("answer"),
        socket::PROTOCOL
    );
    // Closed after the welcome: the spawn either cannot be written or is never
    // answered, and no pane is started.
    let _ = frame::write(&mut stream, &Frame::new(1, Kind::Spawn, shell().encode()));
    assert!(
        matches!(frame::read(&mut stream), Ok(None) | Err(_)),
        "closed after the welcome"
    );
}
