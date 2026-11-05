use std::time::{Duration, Instant};

use super::*;

fn vars(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
    pairs
        .iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect()
}

#[test]
fn only_the_allowed_names_reach_the_daemon() {
    let kept = allowed(vars(&[
        ("HOME", "/home/someone"),
        ("XAUTHORITY", "/run/user/1000/gdm/Xauthority"),
        ("LC_TIME", "pt_BR.UTF-8"),
        ("ANTHROPIC_API_KEY", "sk-secret"),
        ("GITHUB_TOKEN", "ghp-secret"),
        ("LD_LIBRARY_PATH", "/tmp/.mount_x/usr/lib"),
        ("APPIMAGE", "/home/someone/devpit.AppImage"),
    ]));
    let names: Vec<_> = kept
        .iter()
        .map(|(name, _)| name.to_string_lossy())
        .collect();
    // XAUTHORITY: an X11 program started in a pane is refused without it.
    assert_eq!(names, ["HOME", "XAUTHORITY", "LC_TIME"]);
}

/// The daemon's process sees the allowlist and nothing else: a secret set in
/// the desktop does not reach it, nor a shell it starts.
#[test]
fn a_started_daemon_has_a_clean_environment() {
    let dir = tempfile::tempdir().expect("dir");
    let seen = dir.path().join("env");
    let script = format!("env > {}", seen.display());
    start(
        Path::new("/bin/sh"),
        &[OsStr::new("-c"), OsStr::new(&script)],
        vars(&[
            ("HOME", "/home/someone"),
            ("DEVPIT_SECRET_TOKEN", "do-not-pass"),
            ("PATH", "/usr/bin:/bin"),
        ]),
    )
    .expect("start");
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut env = String::new();
    while Instant::now() < deadline {
        env = std::fs::read_to_string(&seen).unwrap_or_default();
        if env.contains("HOME=") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(env.contains("HOME=/home/someone"), "{env}");
    assert!(!env.contains("DEVPIT_SECRET_TOKEN"), "{env}");
}

#[test]
fn the_copy_is_per_protocol_and_one_there_is_kept() {
    let home = tempfile::tempdir().expect("home");
    let source = home.path().join("source");
    std::fs::write(&source, b"first").expect("source");
    let at = install(&source, home.path()).expect("install");
    assert_eq!(at, installed_at(home.path()));
    assert!(at.ends_with(format!("ptyd/v{}/{BINARY}", socket::PROTOCOL)));
    // A newer desktop of the same protocol keeps the copy, which may be the
    // one running.
    std::fs::write(&source, b"second").expect("source");
    install(&source, home.path()).expect("again");
    assert_eq!(std::fs::read(&at).expect("copy"), b"first");
}

#[test]
fn with_no_daemon_and_no_binary_it_is_not_found() {
    let home = tempfile::tempdir().expect("home");
    let at = home.path().join("devpit-ptyd.sock");
    let err = connect_or_start(None, &[], home.path(), &at).expect_err("nothing to start");
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
    let missing = home.path().join("no-such-binary");
    let err = connect_or_start(Some(&missing), &[], home.path(), &at).expect_err("not installed");
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

#[test]
fn a_running_daemon_is_found_without_starting_another() {
    let home = tempfile::tempdir().expect("home");
    let at = home.path().join("devpit-ptyd.sock");
    let listener = socket::listen(&at).expect("listen");
    let daemon = crate::daemon::Daemon::new();
    std::thread::spawn(move || daemon.serve(listener));
    connect_or_start(None, &[], home.path(), &at).expect("found");
    assert!(!installed_at(home.path()).exists(), "nothing was installed");
}
