use super::*;

use devpit_core::home::{settle, ProjectHome};

/// A picture pasted before its project's folder had a name still opens.
///
/// The transcript keeps the path it was pasted at; `allowed` looks it up in
/// the named folder before checking where it may be opened from.
#[test]
fn a_picture_pasted_under_the_old_folder_opens_from_the_new_one() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path();
    let store = Store::open(&home.join("state.db")).expect("store");
    let checkout = home.join("checkouts/demo");
    std::fs::create_dir_all(&checkout).expect("checkout");
    let id = store.add_project(&checkout, None).expect("project");
    let old = devpit_core::home::projects_dir(home)
        .join(&id)
        .join("pasted");
    std::fs::create_dir_all(&old).expect("old folder");
    std::fs::write(old.join("pasted-1.png"), "png").expect("picture");

    settle(&store, home).expect("settle");

    let written = old.join("pasted-1.png").display().to_string();
    let opened = allowed_in(&store, &[], home, &written).expect("opens");
    let pasted = ProjectHome::of(&store, home, &id).expect("home").pasted();
    assert_eq!(
        opened,
        pasted.join("pasted-1.png").canonicalize().expect("real")
    );
}

#[test]
fn a_file_in_a_registered_project_may_be_opened() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("repo");
    std::fs::create_dir_all(project.join("src")).expect("create");
    std::fs::write(project.join("src/main.rs"), "").expect("write");

    let found = openable(
        std::slice::from_ref(&project),
        dir.path(),
        &project.join("src/main.rs"),
    );
    assert!(found.is_some());
}

#[test]
fn a_file_in_the_devpit_workspace_may_be_opened() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    std::fs::create_dir_all(&home).expect("create");
    std::fs::write(home.join("board.db"), "").expect("write");

    assert!(openable(&[], &home, &home.join("board.db")).is_some());
}

#[test]
fn a_file_somewhere_else_entirely_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("repo");
    let home = dir.path().join("home");
    let elsewhere = dir.path().join("private");
    for path in [&project, &home, &elsewhere] {
        std::fs::create_dir_all(path).expect("create");
    }
    std::fs::write(elsewhere.join("keys"), "").expect("write");

    assert!(openable(&[project], &home, &elsewhere.join("keys")).is_none());
}

/// The check resolves before it compares, so a link inside a project that
/// points out of it is refused rather than followed.
#[test]
fn a_symlink_pointing_out_of_the_project_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join("repo");
    let home = dir.path().join("home");
    let elsewhere = dir.path().join("private");
    for path in [&project, &home, &elsewhere] {
        std::fs::create_dir_all(path).expect("create");
    }
    std::fs::write(elsewhere.join("keys"), "").expect("write");
    std::os::unix::fs::symlink(elsewhere.join("keys"), project.join("looks-inside"))
        .expect("symlink");

    assert!(openable(
        std::slice::from_ref(&project),
        &home,
        &project.join("looks-inside")
    )
    .is_none());
}

#[test]
fn a_path_that_is_not_there_is_refused_rather_than_handed_over() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(openable(
        &[dir.path().to_path_buf()],
        dir.path(),
        &dir.path().join("gone")
    )
    .is_none());
}

/// A skill lives in the CLI's configuration directory, which is neither a
/// project nor the devpit workspace.
///
/// The Skills panel lists those files and offers Open, Show in the finder and
/// Copy path. Before the directory was allowed, the first two answered
/// "that path is not in a project or in the devpit workspace" for every skill
/// on the machine — a panel of buttons that had never once worked.
#[test]
fn a_skill_in_the_cli_configuration_may_be_opened() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join(".claude/plugins/cache/omc/skills/tdd");
    let home = dir.path().join("devpit");
    std::fs::create_dir_all(&cli).expect("create");
    std::fs::create_dir_all(&home).expect("create");
    std::fs::write(cli.join("SKILL.md"), "---\nname: tdd\n---\n").expect("write");

    let roots = [dir.path().join(".claude")];
    assert!(openable(&roots, &home, &cli.join("SKILL.md")).is_some());
}

/// Allowing the configuration directory must not allow the home above it.
#[test]
fn the_directory_beside_the_cli_configuration_is_still_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cli = dir.path().join(".claude");
    let home = dir.path().join("devpit");
    for path in [&cli, &home] {
        std::fs::create_dir_all(path).expect("create");
    }
    std::fs::write(dir.path().join(".ssh-key"), "").expect("write");

    assert!(openable(&[cli], &home, &dir.path().join(".ssh-key")).is_none());
}

/// What the window may hand to the browser.
///
/// The link did nothing at all before this: a `target="_blank"` reaches wry's
/// `new_window_req_handler`, which tauri never sets, so nothing acted on the
/// click. Now that something does, the scheme is the whole of the check —
/// sabotage by accepting anything and the four refusals below stop failing.
#[test]
fn only_a_web_address_is_handed_to_the_browser() {
    // `RpcError` has no `PartialEq`, so the answer is read rather than compared.
    assert_eq!(
        web_address("https://github.com/jholhewres/devpit/releases/latest").ok(),
        Some("https://github.com/jholhewres/devpit/releases/latest")
    );
    assert_eq!(
        web_address("https://example.org").ok(),
        Some("https://example.org")
    );

    // A scheme that does something other than open a page.
    assert!(web_address("http://example.org").is_err(), "plain http");
    assert!(web_address("file:///etc/passwd").is_err(), "a local file");
    assert!(
        web_address("javascript:alert(1)").is_err(),
        "a script, not an address"
    );
    // A flag rather than an address.
    assert!(web_address("-fWhatever").is_err());
    // And the two ways a host lies about where it goes.
    assert!(web_address("https://").is_err(), "no host");
    assert!(web_address("https://user:pw@example.org").is_err());
    assert!(web_address("https://example.org/a b").is_err(), "a space");
}
