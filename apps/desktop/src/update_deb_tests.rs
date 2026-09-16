//! What a downloaded package has to still be, and what the command may say.

use super::*;

/// A minimal thing that starts like a Debian package.
fn a_deb() -> Vec<u8> {
    let mut bytes = DEBIAN_MAGIC.to_vec();
    bytes.extend_from_slice(b"   0           0     0     100644  4         `\n2.0\n");
    bytes
}

#[test]
fn the_digest_is_the_sha_256_of_the_bytes() {
    assert_eq!(
        digest_of(b"one"),
        "7692c3ad3540bb803c020b3aee66cd8887123234ea0c6e7143c0add73ff431ed"
    );
}

#[test]
fn a_package_that_is_still_itself_is_accepted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("devpit_0.2.0_amd64.deb");
    let bytes = a_deb();
    std::fs::write(&path, &bytes).expect("write");

    assert_eq!(still_ours(&path, dir.path(), &digest_of(&bytes)), Ok(()));
}

/// Five ways it is not, and five different things to say about it.
#[test]
fn every_way_a_package_stops_being_ours() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bytes = a_deb();
    let digest = digest_of(&bytes);

    let gone = dir.path().join("gone.deb");
    assert_eq!(
        still_ours(&gone, dir.path(), &digest),
        Err(NotOurs::Missing)
    );

    let elsewhere = dir.path().join("under");
    std::fs::create_dir_all(&elsewhere).expect("dir");
    let outside = elsewhere.join("devpit.deb");
    std::fs::write(&outside, &bytes).expect("write");
    assert_eq!(
        still_ours(&outside, dir.path(), &digest),
        Err(NotOurs::Elsewhere)
    );

    let changed = dir.path().join("changed.deb");
    std::fs::write(&changed, b"something else entirely").expect("write");
    assert_eq!(
        still_ours(&changed, dir.path(), &digest),
        Err(NotOurs::HashMismatch)
    );

    // The one the plugin's own installer detection can cause: an AppImage,
    // whole and unaltered, is still not a package.
    let appimage = dir.path().join("not.deb");
    let other = b"\x7fELF and the rest of an AppImage".to_vec();
    std::fs::write(&appimage, &other).expect("write");
    assert_eq!(
        still_ours(&appimage, dir.path(), &digest_of(&other)),
        Err(NotOurs::NotADeb)
    );

    #[cfg(unix)]
    {
        let link = dir.path().join("linked.deb");
        std::os::unix::fs::symlink(&changed, &link).expect("link");
        assert_eq!(
            still_ours(&link, dir.path(), &digest),
            Err(NotOurs::NotRegular)
        );
    }

    // And each says something of its own.
    let said: std::collections::HashSet<&str> = [
        NotOurs::Missing,
        NotOurs::NotRegular,
        NotOurs::Elsewhere,
        NotOurs::HashMismatch,
        NotOurs::NotADeb,
    ]
    .iter()
    .map(|why| why.said())
    .collect();
    assert_eq!(said.len(), 5);
}

/// The command is built from binaries where only root writes, never `$PATH`.
#[test]
fn the_command_names_binaries_from_the_trusted_directories() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    for tool in ["sudo", "apt"] {
        std::fs::write(bin.join(tool), "#!/bin/sh\n").expect("tool");
    }
    let dirs = [bin.to_str().expect("utf-8")];
    let package = dir.path().join("devpit_0.2.0_amd64.deb");

    let line = install_command(&package, &dirs).expect("a command");

    assert!(
        line.starts_with(&format!("{}/sudo ", bin.display())),
        "{line}"
    );
    assert!(
        line.contains(&format!("{}/apt install ", bin.display())),
        "{line}"
    );
    assert!(
        line.ends_with(&format!("'{}'", package.display())),
        "{line}"
    );
    assert!(
        !line.contains("-y"),
        "the person has to see and agree: {line}"
    );
}

#[test]
fn dpkg_stands_in_for_apt_and_a_machine_with_neither_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin");
    std::fs::create_dir_all(&bin).expect("bin");
    std::fs::write(bin.join("sudo"), "#!/bin/sh\n").expect("tool");
    let dirs = [bin.to_str().expect("utf-8")];
    let package = dir.path().join("devpit.deb");

    assert!(install_command(&package, &dirs)
        .expect_err("apt was found where there is none")
        .contains("no apt or dpkg"));

    std::fs::write(bin.join("dpkg"), "#!/bin/sh\n").expect("tool");
    let line = install_command(&package, &dirs).expect("a command");
    assert!(line.contains("dpkg -i "), "{line}");

    // And with no sudo there is no command to give at all.
    let bare = tempfile::tempdir().expect("tempdir");
    let empty = [bare.path().to_str().expect("utf-8")];
    assert!(install_command(&package, &empty)
        .expect_err("a command with no sudo")
        .contains("no sudo"));
}

#[test]
fn a_path_that_cannot_be_typed_is_refused() {
    let dirs = TRUSTED;
    assert!(install_command(Path::new("devpit.deb"), &dirs)
        .expect_err("a relative path was allowed")
        .contains("absolute"));
    assert!(install_command(Path::new("/tmp/it's.deb"), &dirs)
        .expect_err("a quote was allowed")
        .contains("cannot be typed"));
}
