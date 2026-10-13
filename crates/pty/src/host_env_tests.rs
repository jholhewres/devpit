use super::*;

const MOUNTS: &str = "/tmp/.mount_";

fn vars(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect()
}

fn change(name: &str, value: Option<&str>) -> Change {
    (name.to_owned(), value.map(str::to_owned))
}

#[test]
fn the_mounts_are_found_beside_the_appdir() {
    assert_eq!(mounts_under("/tmp/.mount_devpitBhkiPC"), MOUNTS);
    assert_eq!(mounts_under("/var/tmp/.mount_devpitX/"), "/var/tmp/.mount_");
}

/// Recorded from a shell inside the 0.1.7 AppImage, where `python3` failed
/// with "No module named 'encodings'".
#[test]
fn a_single_path_into_the_mount_is_removed() {
    let changes = cleaned(
        vars(&[
            ("PYTHONHOME", "/tmp/.mount_devpitBhkiPC/usr/"),
            (
                "GTK_IM_MODULE_FILE",
                "/tmp/.mount_devpitBhkiPC//usr/lib/gtk-3.0/3.0.0/immodules.cache",
            ),
        ]),
        MOUNTS,
    );
    assert_eq!(
        changes,
        vec![
            change("PYTHONHOME", None),
            change("GTK_IM_MODULE_FILE", None)
        ]
    );
}

/// What the person had before the AppImage prepended its own survives.
#[test]
fn a_path_list_keeps_what_was_not_in_the_mount() {
    let changes = cleaned(
        vars(&[
            (
                "XDG_DATA_DIRS",
                "/tmp/.mount_devpitA/usr/share/:/usr/share:/var/lib/flatpak/exports/share",
            ),
            (
                "LD_LIBRARY_PATH",
                "/tmp/.mount_devpitA/usr/lib/:/tmp/.mount_devpitA/usr/lib/x86_64-linux-gnu/",
            ),
            ("PYTHONPATH", "/tmp/.mount_devpitA/usr/share/pyshared/:"),
        ]),
        MOUNTS,
    );
    assert_eq!(
        changes,
        vec![
            change(
                "XDG_DATA_DIRS",
                Some("/usr/share:/var/lib/flatpak/exports/share")
            ),
            change("LD_LIBRARY_PATH", None),
            change("PYTHONPATH", None),
        ]
    );
}

/// A tmux server started by an earlier devpit holds the mount that devpit had.
#[test]
fn a_mount_that_has_since_moved_goes_too() {
    let changes = cleaned(
        vars(&[("PERLLIB", "/tmp/.mount_devpitOLD123/usr/share/perl5/")]),
        MOUNTS,
    );
    assert_eq!(changes, vec![change("PERLLIB", None)]);
}

#[test]
fn what_never_named_the_mount_is_left_alone() {
    let changes = cleaned(
        vars(&[
            ("PATH", "/home/me/.local/bin:/usr/bin"),
            ("GTK_THEME", "Adwaita:dark"),
            ("HOME", "/home/me"),
        ]),
        MOUNTS,
    );
    assert!(changes.is_empty(), "{changes:?}");
}

/// A devpit started from one of these terminals must not take itself for the
/// installed AppImage.
#[test]
fn the_names_of_the_appimage_itself_are_removed() {
    let changes = cleaned(
        vars(&[
            ("APPIMAGE", "/home/me/.local/bin/devpit"),
            ("ARGV0", "devpit"),
        ]),
        MOUNTS,
    );
    assert_eq!(
        changes,
        vec![change("APPIMAGE", None), change("ARGV0", None)]
    );
}

#[cfg(unix)]
#[test]
fn a_program_is_found_where_the_path_has_it_executable() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let (first, second) = (dir.path().join("a"), dir.path().join("b"));
    std::fs::create_dir_all(&first).expect("a");
    std::fs::create_dir_all(&second).expect("b");
    // Not executable in the first folder: skipped, as a shell would.
    std::fs::write(first.join("tool"), "").expect("plain");
    std::fs::write(second.join("tool"), "#!/bin/sh\n").expect("tool");
    std::fs::set_permissions(second.join("tool"), std::fs::Permissions::from_mode(0o755))
        .expect("chmod");
    let path = std::env::join_paths(["relative/bin".into(), first, second.clone()]).expect("path");
    assert_eq!(found_on(&path, "tool".as_ref()), Some(second.join("tool")));
    assert_eq!(found_on(&path, "missing".as_ref()), None);
}

#[cfg(unix)]
#[test]
fn a_command_is_spawned_by_its_path_so_it_needs_no_fork() {
    let sh = command("sh");
    assert!(
        Path::new(sh.get_program()).is_absolute(),
        "{:?}",
        sh.get_program()
    );
    let missing = command("devpit-no-such-program");
    assert_eq!(missing.get_program(), "devpit-no-such-program");
    let given = command("/bin/sh");
    assert_eq!(given.get_program(), "/bin/sh");
}
