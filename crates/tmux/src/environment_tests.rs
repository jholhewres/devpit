use super::*;

/// The shape `show-environment -g` prints, from a server an AppImage started.
#[test]
fn the_listing_reads_as_pairs_and_skips_removals() {
    let listed = "HOME=/home/me\n-DISPLAY\nPYTHONHOME=/tmp/.mount_devpitA/usr/\n";
    assert_eq!(
        global(listed),
        vec![
            ("HOME".to_owned(), "/home/me".to_owned()),
            (
                "PYTHONHOME".to_owned(),
                "/tmp/.mount_devpitA/usr/".to_owned()
            ),
        ]
    );
}

#[test]
fn the_changes_go_to_the_server_in_one_invocation() {
    let changes = vec![
        ("PYTHONHOME".to_owned(), None),
        ("XDG_DATA_DIRS".to_owned(), Some("/usr/share".to_owned())),
    ];
    assert_eq!(
        one_line(&changes),
        [
            "set-environment",
            "-g",
            "-u",
            "PYTHONHOME",
            ";",
            "set-environment",
            "-g",
            "XDG_DATA_DIRS",
            "/usr/share",
        ]
    );
}
