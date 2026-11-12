use std::os::unix::fs::PermissionsExt;

use super::*;

/// A temporary directory a socket may be made in whatever the umask: its
/// user's alone, as `$XDG_RUNTIME_DIR` and devpit's home are.
pub(crate) fn private_tempdir() -> tempfile::TempDir {
    tempfile::Builder::new()
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .expect("dir")
}

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
    let dir = private_tempdir();
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

/// Without a runtime dir the socket's directory is made owner-only, whatever
/// the umask: that it is the user's alone is what keeps the socket theirs.
#[test]
fn a_directory_made_for_the_socket_is_owner_only() {
    let dir = tempfile::tempdir().expect("dir");
    let home = dir.path().join("home");
    let listener = listen(&home.join(NAME)).expect("listen");
    let mode = std::fs::metadata(&home).expect("home").permissions().mode();
    drop(listener);
    assert_eq!(mode & 0o777, 0o700);
}

/// A directory others can write to is refused: they could put their own
/// socket where this one is.
#[test]
fn a_directory_others_can_write_to_is_refused() {
    let dir = tempfile::tempdir().expect("dir");
    std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o775)).expect("chmod");
    let err = listen(&dir.path().join(NAME)).expect_err("refused");
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn only_a_directory_its_user_alone_can_write_to_is_private() {
    assert!(private_dir(1000, 0o40700, 1000));
    assert!(
        private_dir(1000, 0o40755, 1000),
        "others may read, not write"
    );
    assert!(!private_dir(1000, 0o40775, 1000), "group-writable");
    assert!(!private_dir(1000, 0o40757, 1000), "writable by anyone");
    assert!(!private_dir(0, 0o40700, 1000), "another user's");
}

/// The client asks the kernel who the daemon is, as the daemon asks of it:
/// a socket of another user's is refused before a byte is written to it.
#[test]
fn a_client_refuses_a_daemon_of_another_user() {
    let dir = private_tempdir();
    let at = dir.path().join(NAME);
    let listener = listen(&at).expect("listen");
    let err = connect_as(&at, own_uid().wrapping_add(1)).expect_err("refused");
    drop(listener);
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
}

/// Only a socket is taken for a dead daemon's: anything else at that path is
/// somebody's file, and is left where it is.
#[test]
fn a_file_that_is_not_a_socket_is_never_removed() {
    let dir = private_tempdir();
    let at = dir.path().join("devpit-ptyd.sock");
    std::fs::write(&at, "notes").expect("a file");
    assert!(listen(&at).is_err());
    assert_eq!(std::fs::read_to_string(&at).expect("kept"), "notes");
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
