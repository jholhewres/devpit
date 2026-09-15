use super::*;

fn nothing() -> impl FnMut(&str) {
    |_| {}
}

#[test]
fn a_worktree_with_nothing_declared_is_left_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let done = run(&Prime::default(), dir.path(), dir.path(), nothing()).expect("prime");
    assert_eq!(done, Primed::Nothing);
}

#[test]
fn a_declared_link_points_at_the_main_checkout_rather_than_copying_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let main = dir.path().join("main");
    let side = dir.path().join("side");
    std::fs::create_dir_all(&main).expect("create");
    std::fs::create_dir_all(&side).expect("create");
    std::fs::write(main.join(".env"), "SECRET=1\n").expect("write");

    let prime = Prime {
        link: vec![".env".to_owned()],
        ..Prime::default()
    };
    assert_eq!(
        run(&prime, &main, &side, nothing()).expect("prime"),
        Primed::Done
    );

    let link = side.join(".env");
    assert!(
        std::fs::symlink_metadata(&link)
            .expect("metadata")
            .file_type()
            .is_symlink(),
        "the .env was copied rather than linked"
    );
    // Changing the original is visible through the link: that is the point.
    std::fs::write(main.join(".env"), "SECRET=2\n").expect("write");
    assert_eq!(std::fs::read_to_string(&link).expect("read"), "SECRET=2\n");
}

#[test]
fn a_shared_variable_reaches_the_command_as_a_variable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prime = Prime {
        share: [("CARGO_TARGET_DIR".to_owned(), "/tmp/shared".to_owned())]
            .into_iter()
            .collect(),
        run: vec!["printenv CARGO_TARGET_DIR > seen".to_owned()],
        ..Prime::default()
    };
    assert_eq!(
        run(&prime, dir.path(), dir.path(), nothing()).expect("prime"),
        Primed::Done
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("seen")).expect("read"),
        "/tmp/shared\n"
    );
}

#[test]
fn a_failed_preparation_names_the_command_and_its_code() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prime = Prime {
        run: vec!["exit 3".to_owned()],
        ..Prime::default()
    };
    assert_eq!(
        run(&prime, dir.path(), dir.path(), nothing()).expect("prime"),
        Primed::Failed {
            command: "exit 3".to_owned(),
            code: 3,
        }
    );
}

#[test]
fn a_preparation_runs_once_and_not_again() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prime = Prime {
        run: vec!["echo x >> counted".to_owned()],
        ..Prime::default()
    };
    run(&prime, dir.path(), dir.path(), nothing()).expect("prime");
    run(&prime, dir.path(), dir.path(), nothing()).expect("prime");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("counted")).expect("read"),
        "x\n",
        "the preparation ran twice"
    );
}

#[test]
fn a_command_that_does_not_exist_is_named_when_the_preparation_is_saved() {
    let prime = Prime {
        run: vec!["definitely-not-a-program --go".to_owned()],
        ..Prime::default()
    };
    assert_eq!(missing(&prime), vec!["definitely-not-a-program".to_owned()]);
}

#[test]
fn a_command_that_exists_is_not_named() {
    let prime = Prime {
        run: vec!["sh -c true".to_owned()],
        ..Prime::default()
    };
    assert!(missing(&prime).is_empty());
}

#[test]
fn a_preparation_survives_the_round_trip() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("prime.json");
    let declared = Prime {
        link: vec![".env".to_owned()],
        share: [("A".to_owned(), "b".to_owned())].into_iter().collect(),
        run: vec!["pnpm install".to_owned()],
    };
    write(&path, &declared).expect("write");
    assert_eq!(read(&path), declared);
}
