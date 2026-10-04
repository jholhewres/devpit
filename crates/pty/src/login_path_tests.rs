use super::*;
use std::ffi::OsStr;

#[test]
fn the_path_is_read_past_what_the_shell_config_prints() {
    let said = "welcome back\nsome banner __devpit_path__ no\n__devpit_path__/home/me/.tool/bin:/usr/bin\n";
    assert_eq!(marked(said).as_deref(), Some("/home/me/.tool/bin:/usr/bin"));
    assert_eq!(marked("nothing marked"), None);
}

#[test]
fn the_shell_entries_lead_and_none_is_lost_or_doubled() {
    let path = joined(
        OsStr::new("/home/me/.tool/bin:/usr/bin"),
        OsStr::new("/usr/bin:/snap/bin"),
    );
    assert_eq!(path, OsStr::new("/home/me/.tool/bin:/usr/bin:/snap/bin"));
}

/// The installers' folders: Claude's own, npm, bun, brew on Apple silicon.
#[cfg(unix)]
#[test]
fn the_usual_install_folders_are_known() {
    let home = Path::new("/Users/me");
    let dirs = known_dirs(home);
    for wanted in [
        "/Users/me/.local/bin",
        "/Users/me/.claude/local",
        "/Users/me/.npm-global/bin",
        "/Users/me/.bun/bin",
        "/opt/homebrew/bin",
        "/usr/local/bin",
    ] {
        assert!(
            dirs.contains(&PathBuf::from(wanted)),
            "{wanted} is not searched"
        );
    }
}

/// nvm keeps one folder per Node; the newest by version is the one searched.
#[cfg(unix)]
#[test]
fn the_newest_nvm_node_is_searched() {
    let home = tempfile::tempdir().expect("tempdir");
    for version in ["v18.20.1", "v22.3.0", "v9.11.2"] {
        std::fs::create_dir_all(
            home.path()
                .join(".nvm/versions/node")
                .join(version)
                .join("bin"),
        )
        .expect("dir");
    }
    let dirs = known_dirs(home.path());
    assert!(dirs.contains(&home.path().join(".nvm/versions/node/v22.3.0/bin")));
    assert!(!dirs.contains(&home.path().join(".nvm/versions/node/v9.11.2/bin")));
}

#[test]
fn folders_are_added_after_the_path_once() {
    let path = with_dirs(
        OsStr::new("/usr/bin:/opt/homebrew/bin"),
        &[
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/Users/me/.local/bin"),
        ],
    );
    assert_eq!(
        path,
        OsStr::new("/usr/bin:/opt/homebrew/bin:/Users/me/.local/bin")
    );
}

/// A shell whose config never returns is ended, with what it started.
#[cfg(unix)]
#[test]
fn a_shell_that_hangs_is_given_up_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let shell = dir.path().join("slow-shell");
    std::fs::write(&shell, "#!/bin/sh\nsleep 30\n").expect("write");
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&shell, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    let started = Instant::now();
    let (outcome, path) = ask(shell.to_str().unwrap());
    assert_eq!((outcome, path), ("timed out", None));
    assert!(started.elapsed() < WAITS + Duration::from_secs(2));
}
