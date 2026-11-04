use std::os::unix::fs::PermissionsExt;

use super::*;

#[test]
fn the_socket_is_in_the_runtime_dir_and_else_in_devpits_home() {
    let runtime = tempfile::tempdir().expect("runtime");
    let home = Path::new("/home/someone/.devpit");
    let in_runtime = path(Some(runtime.path()), home);
    assert_eq!(in_runtime.parent(), Some(runtime.path()));
    assert_eq!(path(None, home), home.join(NAME));
    assert_eq!(path(Some(Path::new("relative")), home), home.join(NAME));
    assert_eq!(path(Some(Path::new("/no/such/dir")), home), home.join(NAME));
}

/// Two homes never meet in the runtime dir: not the installed devpit and a
/// development build, not the installed one and an e2e run's home.
#[test]
fn each_home_has_a_socket_of_its_own_and_keeps_it() {
    let runtime = tempfile::tempdir().expect("runtime");
    let release = path(Some(runtime.path()), Path::new("/home/someone/.devpit"));
    let dev = path(Some(runtime.path()), Path::new("/home/someone/.devpit-dev"));
    let e2e = path(
        Some(runtime.path()),
        Path::new("/repo/target/e2e-home/.devpit"),
    );
    assert_ne!(release, dev);
    assert_ne!(release, e2e);
    assert_eq!(
        release,
        path(Some(runtime.path()), Path::new("/home/someone/.devpit")),
        "the same home finds the same daemon on every run"
    );
    assert!(release.as_os_str().len() < 108, "{}", release.display());
}

#[test]
fn the_socket_is_owner_only_and_a_dead_one_is_replaced() {
    let dir = tempfile::tempdir().expect("dir");
    let at = dir.path().join("devpit-ptyd.sock");
    drop(listen(&at).expect("first"));
    // The listener is gone but its file stays: the next daemon replaces it.
    // Gone for certain first: a pane test forking in parallel holds a copy of
    // every open descriptor until its child execs, this listener's included.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while UnixStream::connect(&at).is_ok() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let listener = listen(&at).expect("replaced");
    let mode = std::fs::metadata(&at).expect("socket").permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
    // A live one is not taken over.
    let err = listen(&at).expect_err("in use");
    assert_eq!(err.kind(), io::ErrorKind::AddrInUse);
    drop(listener);
}

#[test]
fn the_kernel_names_the_peer_and_only_the_same_user_is_admitted() {
    let (one, two) = UnixStream::pair().expect("pair");
    let peer = peer_uid(&one).expect("peer uid");
    assert_eq!(peer, own_uid());
    assert!(admitted(peer, own_uid()));
    assert!(!admitted(peer.wrapping_add(1), own_uid()));
    assert!(!admitted(0, 1000), "root is another user too");
    drop(two);
}

#[cfg(target_os = "linux")]
#[test]
fn the_kernel_names_the_peers_pid() {
    let (one, _two) = UnixStream::pair().expect("pair");
    assert_eq!(peer_pid(&one).expect("peer pid"), std::process::id());
}

#[test]
fn the_same_protocol_is_welcomed_and_another_is_told_ours() {
    let (mut daemon, mut client) = UnixStream::pair().expect("pair");
    let greeting = std::thread::spawn(move || greet(&mut daemon));
    assert_eq!(hello(&mut client, PROTOCOL).expect("welcome"), PROTOCOL);
    greeting.join().expect("thread").expect("greeted");

    let (mut daemon, mut client) = UnixStream::pair().expect("pair");
    let greeting = std::thread::spawn(move || greet(&mut daemon));
    assert_eq!(hello(&mut client, PROTOCOL + 1).expect("answer"), PROTOCOL);
    let refused = greeting.join().expect("thread").expect_err("refused");
    assert_eq!(refused.kind(), io::ErrorKind::Unsupported);
}

#[test]
fn a_first_frame_that_is_not_hello_is_refused() {
    let (mut daemon, mut client) = UnixStream::pair().expect("pair");
    let greeting = std::thread::spawn(move || greet(&mut daemon));
    frame::write(&mut client, &Frame::new(1, Kind::Input, b"ls\n".to_vec())).expect("send");
    let refused = greeting.join().expect("thread").expect_err("refused");
    assert_eq!(refused.kind(), io::ErrorKind::InvalidData);
}
