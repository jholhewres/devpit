//! The rules a plugin's files keep, against a real folder in a tempdir.

use std::time::Duration;

use devpit_core::data_files::temp_for;
use devpit_core::home::{projects_dir, ProjectHome};

use super::*;

const PLUGIN: &str = "excalidraw";
const NAME: &str = "flow.excalidraw";

/// A workspace root with its store, one project, and the plugin installed.
fn workspace() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let checkout = dir.path().join("checkout");
    std::fs::create_dir_all(&checkout).expect("checkout");
    let project = store.add_project(&checkout, None).expect("project");
    store.install_plugin(&project, PLUGIN).expect("installed");
    (dir, store, project)
}

/// Where the plugin's files are.
fn data_dir(root: &Path, store: &Store, project: &str) -> PathBuf {
    let home = ProjectHome::of(store, root, project).expect("home");
    home.dir().join(home.plugin_data(PLUGIN).expect("an id"))
}

fn max_bytes() -> f64 {
    manifest(PLUGIN).expect("shipped").data.max_bytes
}

fn on_disk(root: &Path, store: &Store, project: &str) -> String {
    std::fs::read_to_string(data_dir(root, store, project).join(NAME)).expect("the file")
}

#[test]
fn a_written_file_is_listed_and_reads_back() {
    let (dir, store, id) = workspace();
    let root = dir.path();

    let saved = write(&store, root, &id, PLUGIN, NAME, "scene", None).expect("write");

    let back = read(&store, root, &id, PLUGIN, NAME).expect("read");
    assert_eq!(
        (back.text.as_str(), back.modified),
        ("scene", saved.modified)
    );
    let listed = list(&store, root, &id, PLUGIN).expect("list");
    let file = PluginFile {
        name: NAME.to_owned(),
        bytes: 5.0,
        modified: saved.modified,
    };
    assert_eq!(listed.files, [file]);
}

#[test]
fn a_name_that_is_not_one_plain_file_is_invalid() {
    let (dir, store, id) = workspace();
    let names = [
        "a/flow.excalidraw",
        "a\\flow.excalidraw",
        "../flow.excalidraw",
        "flow..excalidraw",
        "/tmp/flow.excalidraw",
        "C:flow.excalidraw",
        "fl\0ow.excalidraw",
        ".flow.excalidraw",
    ];
    for name in names {
        let err = write(&store, dir.path(), &id, PLUGIN, name, "x", None).expect_err(name);
        assert_eq!(err.code, ErrorCode::Invalid, "{name:?}");
    }
    assert!(!data_dir(dir.path(), &store, &id).exists());
}

#[test]
fn an_extension_the_manifest_does_not_declare_is_invalid() {
    let (dir, store, id) = workspace();
    for name in ["flow.txt", "flow", "flow.excalidraw.txt", "flow.EXCALIDRAW"] {
        let err = write(&store, dir.path(), &id, PLUGIN, name, "x", None).expect_err(name);
        assert_eq!(err.code, ErrorCode::Invalid, "{name:?}");
    }
    assert!(!data_dir(dir.path(), &store, &id).exists());
}

#[cfg(unix)]
#[test]
fn a_file_symlinked_out_of_the_plugin_folder_is_forbidden() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    write(&store, root, &id, PLUGIN, NAME, "mine", None).expect("the folder exists");
    let outside = root.join("outside.excalidraw");
    std::fs::write(&outside, "not the plugin's").expect("outside");
    let link = "link.excalidraw";
    std::os::unix::fs::symlink(&outside, data_dir(root, &store, &id).join(link)).expect("link");

    let refused = [
        read(&store, root, &id, PLUGIN, link).map(|_| ()),
        write(
            &store,
            root,
            &id,
            PLUGIN,
            link,
            "overwritten",
            Some(modified(&outside)),
        )
        .map(|_| ()),
        delete(&store, root, &id, PLUGIN, link).map(|_| ()),
    ];
    for answer in refused {
        assert_eq!(answer.expect_err("refused").code, ErrorCode::Forbidden);
    }
    assert_eq!(
        std::fs::read_to_string(&outside).expect("kept"),
        "not the plugin's"
    );
    assert_eq!(
        list(&store, root, &id, PLUGIN).expect("list").files.len(),
        1
    );
}

/// Read, write and delete agree with the listing, which never shows a link.
#[cfg(unix)]
#[test]
fn a_link_that_stays_inside_the_folder_is_forbidden_too() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let saved = write(&store, root, &id, PLUGIN, NAME, "mine", None).expect("write");
    let alias = data_dir(root, &store, &id).join("alias.excalidraw");
    std::os::unix::fs::symlink(NAME, &alias).expect("link");

    let answers = [
        read(&store, root, &id, PLUGIN, "alias.excalidraw").map(|_| ()),
        write(
            &store,
            root,
            &id,
            PLUGIN,
            "alias.excalidraw",
            "theirs",
            Some(saved.modified),
        )
        .map(|_| ()),
        delete(&store, root, &id, PLUGIN, "alias.excalidraw").map(|_| ()),
    ];
    for answer in answers {
        assert_eq!(answer.expect_err("refused").code, ErrorCode::Forbidden);
    }
    assert_eq!(on_disk(root, &store, &id), "mine");
    let still = alias.symlink_metadata().expect("the link stayed");
    assert!(still.file_type().is_symlink());
}

#[cfg(unix)]
#[test]
fn a_plugin_folder_symlinked_out_of_the_project_is_forbidden() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("elsewhere");
    let data = data_dir(root, &store, &id);
    std::fs::create_dir_all(data.parent().expect("data")).expect("parent");
    std::os::unix::fs::symlink(&elsewhere, &data).expect("link");

    let listed = list(&store, root, &id, PLUGIN).expect_err("refused");
    assert_eq!(listed.code, ErrorCode::Forbidden);
    let written = write(&store, root, &id, PLUGIN, NAME, "x", None).expect_err("refused");
    assert_eq!(written.code, ErrorCode::Forbidden);
    assert_eq!(std::fs::read_dir(&elsewhere).expect("listing").count(), 0);
}

/// Containment is measured from `projects/`, not from the project's folder,
/// or a folder that is itself a link would vouch for wherever it points.
#[cfg(unix)]
#[test]
fn a_project_folder_symlinked_out_of_the_workspace_is_forbidden() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    // A home directory, say, already holding a file the plugin would list.
    let elsewhere = root.join("elsewhere");
    let theirs = elsewhere.join("data").join(PLUGIN);
    std::fs::create_dir_all(&theirs).expect("elsewhere");
    std::fs::write(theirs.join(NAME), "not the plugin's").expect("a file");
    std::fs::create_dir_all(projects_dir(root)).expect("projects");
    let home = ProjectHome::of(&store, root, &id).expect("home");
    std::os::unix::fs::symlink(&elsewhere, home.dir()).expect("link");

    let answers = [
        list(&store, root, &id, PLUGIN).map(|_| ()),
        read(&store, root, &id, PLUGIN, NAME).map(|_| ()),
        write(&store, root, &id, PLUGIN, "new.excalidraw", "x", None).map(|_| ()),
        delete(&store, root, &id, PLUGIN, NAME).map(|_| ()),
    ];
    // The tempdir's own name is part of every absolute path under it.
    let spelled = root.file_name().expect("a name").to_string_lossy();
    for answer in answers {
        let err = answer.expect_err("refused");
        assert_eq!(err.code, ErrorCode::Forbidden, "{}", err.message);
        assert!(
            !err.message.contains(&*spelled),
            "a path reached the page: {}",
            err.message
        );
    }
    assert_eq!(
        std::fs::read_to_string(theirs.join(NAME)).expect("kept"),
        "not the plugin's"
    );
    assert!(!theirs.join("new.excalidraw").exists());
}

/// On another thread: opening a FIFO waits for a writer, and a test that
/// hangs says less than one that fails.
#[cfg(unix)]
#[test]
fn a_fifo_named_like_a_file_is_refused_without_being_opened() {
    let (dir, store, id) = workspace();
    let root = dir.path().to_path_buf();
    write(&store, &root, &id, PLUGIN, NAME, "x", None).expect("the folder exists");
    let fifo = data_dir(&root, &store, &id).join("pipe.excalidraw");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("mkfifo to run");
    assert!(made.success(), "mkfifo made no pipe");

    let (sent, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let store = Store::open(&root.join("state.db")).expect("open");
        let _ = sent.send(read(&store, &root, &id, PLUGIN, "pipe.excalidraw").map(|_| ()));
    });
    let err = answer
        .recv_timeout(Duration::from_secs(10))
        .expect("the read waited on the FIFO")
        .expect_err("refused");
    assert_eq!(err.code, ErrorCode::Invalid, "{}", err.message);
    drop(dir);
}

#[test]
fn an_unknown_plugin_is_not_found() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let answers = [
        list(&store, root, &id, "nope").map(|_| ()),
        read(&store, root, &id, "nope", NAME).map(|_| ()),
        write(&store, root, &id, "nope", NAME, "x", None).map(|_| ()),
        delete(&store, root, &id, "nope", NAME).map(|_| ()),
    ];
    for answer in answers {
        assert_eq!(answer.expect_err("refused").code, ErrorCode::NotFound);
    }
}

#[test]
fn a_plugin_that_is_off_is_forbidden_and_its_files_stay() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let saved = write(&store, root, &id, PLUGIN, NAME, "mine", None).expect("write");
    store.set_plugin_enabled(&id, PLUGIN, false).expect("off");

    let answers = [
        list(&store, root, &id, PLUGIN).map(|_| ()),
        read(&store, root, &id, PLUGIN, NAME).map(|_| ()),
        write(
            &store,
            root,
            &id,
            PLUGIN,
            NAME,
            "theirs",
            Some(saved.modified),
        )
        .map(|_| ()),
        delete(&store, root, &id, PLUGIN, NAME).map(|_| ()),
    ];
    for answer in answers {
        assert_eq!(answer.expect_err("refused").code, ErrorCode::Forbidden);
    }
    assert_eq!(on_disk(root, &store, &id), "mine");
}

/// Uninstalled with its files kept: they stay, and nothing reaches them.
#[test]
fn a_plugin_that_is_not_installed_is_forbidden_and_its_files_stay() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let card = card_in(&store, &id);
    let saved = write(&store, root, &id, PLUGIN, NAME, "mine", None).expect("write");
    crate::plugins::uninstall(&store, root, &id, PLUGIN, false).expect("uninstall");

    let answers = [
        list(&store, root, &id, PLUGIN).map(|_| ()),
        read(&store, root, &id, PLUGIN, NAME).map(|_| ()),
        write(
            &store,
            root,
            &id,
            PLUGIN,
            NAME,
            "theirs",
            Some(saved.modified),
        )
        .map(|_| ()),
        delete(&store, root, &id, PLUGIN, NAME).map(|_| ()),
        pin(&store, root, &id, PLUGIN, NAME, &card),
    ];
    for answer in answers {
        let err = answer.expect_err("refused");
        assert_eq!(err.code, ErrorCode::Forbidden, "{}", err.message);
        assert!(err.message.contains("not installed"), "{}", err.message);
    }
    assert_eq!(on_disk(root, &store, &id), "mine");
    assert_eq!(pins_on(&store, &card), 0);
}

#[test]
fn a_write_past_the_ceiling_is_invalid_and_touches_nothing() {
    let (dir, store, id) = workspace();
    let text = "a".repeat(max_bytes() as usize + 1);

    let err = write(&store, dir.path(), &id, PLUGIN, NAME, &text, None).expect_err("too big");

    assert_eq!(err.code, ErrorCode::Invalid);
    assert!(!data_dir(dir.path(), &store, &id).exists());
}

#[test]
fn a_file_past_the_ceiling_is_invalid_to_read() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    write(&store, root, &id, PLUGIN, NAME, "x", None).expect("the folder exists");
    let big = "big.excalidraw";
    let file = std::fs::File::create(data_dir(root, &store, &id).join(big)).expect("create");
    file.set_len(max_bytes() as u64 + 1).expect("grow");

    let err = read(&store, root, &id, PLUGIN, big).expect_err("too big");
    assert_eq!(err.code, ErrorCode::Invalid, "{}", err.message);
}

/// Fails every read: reaching it means the reader went past the ceiling.
struct Beyond;

impl Read for Beyond {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("read past the ceiling"))
    }
}

#[test]
fn a_read_stops_one_byte_past_the_ceiling() {
    let reader = std::io::repeat(b'a').take(9).chain(Beyond);
    let err = text_within(reader, 8, NAME).expect_err("too big");
    assert_eq!(err.code, ErrorCode::Invalid, "{}", err.message);
}

/// NaN and infinity reach no ceiling arithmetic, even from a release build
/// that only logged the catalogue.
#[test]
fn a_ceiling_that_is_no_byte_count_is_refused_on_use() {
    let mut broken = manifest(PLUGIN).expect("shipped");
    assert_eq!(ceiling_of(&broken).expect("shipped"), max_bytes() as u64);
    for max_bytes in [f64::NAN, f64::INFINITY, -1.0, 0.5] {
        broken.data.max_bytes = max_bytes;
        let err = ceiling_of(&broken).expect_err("refused");
        assert_eq!(err.code, ErrorCode::Internal, "{max_bytes}");
    }
}

#[test]
fn a_listing_reads_no_more_entries_than_its_cap() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    for name in ["a.excalidraw", "b.excalidraw", "c.excalidraw"] {
        write(&store, root, &id, PLUGIN, name, "x", None).expect("write");
    }
    let manifest = manifest(PLUGIN).expect("shipped");

    let listed = files_in(&data_dir(root, &store, &id), &manifest, 2).expect("list");

    assert_eq!(listed.files.len(), 2);
}

#[test]
fn a_new_file_is_refused_where_one_exists() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    write(&store, root, &id, PLUGIN, NAME, "first", None).expect("create");

    let err = write(&store, root, &id, PLUGIN, NAME, "second", None).expect_err("exists");

    assert_eq!(err.code, ErrorCode::Conflict);
    assert_eq!(on_disk(root, &store, &id), "first");
}

#[test]
fn a_write_on_a_stale_read_is_a_conflict() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let saved = write(&store, root, &id, PLUGIN, NAME, "first", None).expect("create");

    let stale = Some(saved.modified - 1000.0);
    let err = write(&store, root, &id, PLUGIN, NAME, "second", stale).expect_err("stale");
    assert_eq!(err.code, ErrorCode::Conflict);
    assert_eq!(on_disk(root, &store, &id), "first");

    let gone = write(
        &store,
        root,
        &id,
        PLUGIN,
        "gone.excalidraw",
        "x",
        Some(saved.modified),
    );
    assert_eq!(gone.expect_err("no such file").code, ErrorCode::Conflict);

    write(
        &store,
        root,
        &id,
        PLUGIN,
        NAME,
        "second",
        Some(saved.modified),
    )
    .expect("current");
    assert_eq!(on_disk(root, &store, &id), "second");
}

#[test]
fn a_write_that_fails_leaves_the_original_and_no_partial_file() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let saved = write(&store, root, &id, PLUGIN, NAME, "original", None).expect("create");
    let data = data_dir(root, &store, &id);
    let temp = temp_for(&data, NAME);
    std::fs::create_dir(&temp).expect("a directory where the write goes first");

    let written = write(
        &store,
        root,
        &id,
        PLUGIN,
        NAME,
        "replacement",
        Some(saved.modified),
    );

    assert_eq!(written.expect_err("failed").code, ErrorCode::Internal);
    assert_eq!(on_disk(root, &store, &id), "original");
    let files: Vec<String> = std::fs::read_dir(&data)
        .expect("listing")
        .flatten()
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(files, [NAME]);
    assert!(temp.is_dir());
}

/// A card on the board of `project`.
fn card_in(store: &Store, project: &str) -> String {
    store.ensure_board(project).expect("board");
    let column = store.columns(project).expect("columns")[0].id.clone();
    store
        .create_card(project, &column, "a card", "")
        .expect("card")
}

fn pins_on(store: &Store, card: &str) -> usize {
    store.attachments(card).expect("pins").len()
}

#[test]
fn a_pinned_drawing_reaches_the_card_with_its_plugin() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let card = card_in(&store, &id);
    write(&store, root, &id, PLUGIN, NAME, "scene", None).expect("write");

    pin(&store, root, &id, PLUGIN, NAME, &card).expect("pin");

    let rows = store.attachments(&card).expect("pins");
    let pinned: Vec<_> = rows.into_iter().map(crate::cards::pinned_now).collect();
    let file = std::fs::canonicalize(data_dir(root, &store, &id).join(NAME)).expect("the file");
    let path = file.display().to_string();
    assert_eq!(pinned.len(), 1);
    let one = &pinned[0];
    assert_eq!(
        (one.plugin.as_deref(), one.label.as_str(), one.path.as_str()),
        (Some(PLUGIN), "flow", path.as_str())
    );
    assert!(one.exists);
}

#[test]
fn pinning_a_name_that_is_not_on_disk_is_not_found() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let card = card_in(&store, &id);

    let no_folder = pin(&store, root, &id, PLUGIN, NAME, &card).expect_err("no folder");
    write(&store, root, &id, PLUGIN, "other.excalidraw", "x", None).expect("write");
    let no_file = pin(&store, root, &id, PLUGIN, NAME, &card).expect_err("no file");

    assert_eq!(no_folder.code, ErrorCode::NotFound, "{}", no_folder.message);
    assert_eq!(no_file.code, ErrorCode::NotFound, "{}", no_file.message);
    assert_eq!(pins_on(&store, &card), 0);
}

#[test]
fn pinning_while_the_plugin_is_off_is_forbidden() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let card = card_in(&store, &id);
    write(&store, root, &id, PLUGIN, NAME, "scene", None).expect("write");
    store.set_plugin_enabled(&id, PLUGIN, false).expect("off");

    let err = pin(&store, root, &id, PLUGIN, NAME, &card).expect_err("off");

    assert_eq!(err.code, ErrorCode::Forbidden, "{}", err.message);
    assert_eq!(pins_on(&store, &card), 0);
}

/// The same answer as a card that does not exist, so a pin says nothing
/// about another project's board.
#[test]
fn a_card_of_another_project_is_not_found() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    write(&store, root, &id, PLUGIN, NAME, "scene", None).expect("write");
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("elsewhere");
    let other = store.add_project(&elsewhere, None).expect("project");
    let theirs = card_in(&store, &other);

    let err = pin(&store, root, &id, PLUGIN, NAME, &theirs).expect_err("theirs");
    let unknown = pin(&store, root, &id, PLUGIN, NAME, "card_nope").expect_err("unknown");

    assert_eq!(err.code, ErrorCode::NotFound, "{}", err.message);
    assert_eq!(err.message, unknown.message);
    assert_eq!(pins_on(&store, &theirs), 0);
}

#[cfg(unix)]
#[test]
fn a_linked_file_is_never_pinned() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    let card = card_in(&store, &id);
    write(&store, root, &id, PLUGIN, NAME, "mine", None).expect("write");
    let outside = root.join("outside.excalidraw");
    std::fs::write(&outside, "not the plugin's").expect("outside");
    let data = data_dir(root, &store, &id);
    std::os::unix::fs::symlink(&outside, data.join("out.excalidraw")).expect("link out");
    std::os::unix::fs::symlink(NAME, data.join("alias.excalidraw")).expect("link in");

    for link in ["out.excalidraw", "alias.excalidraw"] {
        let err = pin(&store, root, &id, PLUGIN, link, &card).expect_err(link);
        assert_eq!(err.code, ErrorCode::Forbidden, "{link}: {}", err.message);
    }
    assert_eq!(pins_on(&store, &card), 0);
}

#[test]
fn a_delete_says_whether_there_was_a_file() {
    let (dir, store, id) = workspace();
    let root = dir.path();
    write(&store, root, &id, PLUGIN, NAME, "x", None).expect("create");

    assert!(
        delete(&store, root, &id, PLUGIN, NAME)
            .expect("delete")
            .removed
    );
    assert!(!data_dir(root, &store, &id).join(NAME).exists());
    assert!(
        !delete(&store, root, &id, PLUGIN, NAME)
            .expect("again")
            .removed
    );
}

/// The screen calls these capabilities; a refusal it draws says the same.
#[test]
fn a_refusal_the_person_reads_never_says_plugin() {
    use devpit_core::home::HomeError;

    let at = PathBuf::from("/nowhere");
    let unreadable = |kind: ErrorKind| TreeError::Unreadable {
        path: at.clone(),
        source: kind.into(),
    };
    let mut refusals = vec![
        refusal("Excalidraw", TreeError::Outside { path: at.clone() }),
        refusal("Excalidraw", TreeError::AlreadyExists { path: at.clone() }),
        refusal("Excalidraw", unreadable(ErrorKind::NotFound)),
        refusal("Excalidraw", unreadable(ErrorKind::PermissionDenied)),
        manifest("nope").expect_err("not shipped"),
        crate::projects::home_refusal(HomeError::PluginId),
        crate::projects::home_refusal(HomeError::PluginElsewhere),
        crate::projects::home_refusal(HomeError::Removal(ErrorKind::Other.into())),
    ];
    #[cfg(unix)]
    {
        let dir = tempfile::tempdir().expect("tempdir");
        std::os::unix::fs::symlink(dir.path(), dir.path().join(NAME)).expect("link");
        refusals.push(file_at(dir.path(), NAME).expect_err("a link"));
    }
    for err in refusals {
        let said = err.message.to_lowercase();
        assert!(!said.contains("plugin"), "{}", err.message);
    }
}
