//! What a project's folder has to keep straight.

use super::*;

const ID: &str = "prj_01M24GHNGDMZCXRFWEDVK387KM";

/// A workspace root in a tempdir, with its store where the app keeps it.
fn workspace() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

/// A project registered from a checkout called `name`.
fn project(root: &Path, store: &Store, name: &str) -> String {
    let checkout = root.join("checkouts").join(name);
    std::fs::create_dir_all(&checkout).expect("checkout");
    store.add_project(&checkout, None).expect("add")
}

fn folder(store: &Store, id: &str) -> String {
    store
        .project_folder(id)
        .expect("read")
        .expect("a row")
        .expect("a folder")
}

fn set_folder(store: &Store, folder: &str) {
    store
        .conn()
        .execute("UPDATE project SET folder = ?1", [folder])
        .expect("a folder from elsewhere");
}

/// The files a build before named folders left under the project's id.
fn legacy(root: &Path, id: &str) -> PathBuf {
    let old = projects_dir(root).join(id);
    std::fs::create_dir_all(old.join("sessions")).expect("legacy folder");
    std::fs::write(old.join("sessions/conv_1.jsonl"), "said\n").expect("transcript");
    old
}

#[test]
fn a_name_and_an_id_make_the_folder() {
    assert_eq!(
        folder_for("Demo Project", ID, |_| false),
        "demo-project-k387km"
    );
}

#[test]
fn accents_are_spelled_without_them() {
    assert!(folder_for("Projeto São João", ID, |_| false).starts_with("projeto-sao-joao-"));
}

#[test]
fn a_name_with_nothing_to_spell_is_a_project() {
    assert!(folder_for("!!!", ID, |_| false).starts_with("project-"));
}

#[test]
fn punctuation_between_words_is_one_hyphen() {
    assert_eq!(
        folder_for("  --My   App!! ", ID, |_| false),
        "my-app-k387km"
    );
}

#[test]
fn a_long_name_is_cut_without_a_hyphen_left_at_the_end() {
    // Sixty characters, the space at the fortieth: the cut at forty lands
    // exactly on the hyphen that space becomes.
    let name = format!("{} {}", "a".repeat(39), "b".repeat(20));
    let made = folder_for(&name, ID, |_| false);
    let slug = made.strip_suffix("-k387km").expect("the suffix");
    assert!(slug.len() <= 40, "{slug}");
    assert!(!slug.ends_with('-'), "{slug}");
}

#[test]
fn a_taken_suffix_grows_to_eight() {
    let made = folder_for("Demo Project", ID, |one| one == "demo-project-k387km");
    assert_eq!(made, "demo-project-dvk387km");
}

/// An id ending in `_` once gave every such row the folder `demo-`, and the
/// unique index then refused them all.
#[test]
fn an_id_too_short_to_name_a_folder_by_still_gets_one_of_its_own() {
    for (odd, other) in [("prj_", "prj2_"), ("prj_ab", "prj_cd"), ("x", "y")] {
        let made = folder_for("Demo", odd, |_| false);
        assert!(
            ProjectHome::at(Path::new("/nowhere"), odd, &made).is_ok(),
            "{odd:?} was given {made:?}, which it cannot open"
        );
        assert_ne!(made, folder_for("Demo", other, |_| false));
    }
}

/// A forty-character slug and a 59-character ULID: the whole id would make a
/// folder of 100, past the 80 a stored folder may have.
#[test]
fn a_long_id_and_a_long_name_still_make_a_folder_it_can_open() {
    let id = format!("prj_{}", "7".repeat(59));
    let short_ones_taken = |one: &str| one.len() < 60;

    let made = folder_for(&"a".repeat(60), &id, short_ones_taken);

    assert!(made.len() <= 80, "{made}");
    assert!(
        ProjectHome::at(Path::new("/nowhere"), &id, &made).is_ok(),
        "{made}"
    );
}

#[test]
fn a_new_project_is_born_with_its_folder() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    assert_eq!(
        folder(&store, &id),
        folder_for("Demo Project", &id, |_| false)
    );
}

#[test]
fn naming_the_unnamed_rows_twice_changes_nothing() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    store
        .conn()
        .execute("UPDATE project SET folder = NULL", [])
        .expect("a row from before migration 12");

    assert_eq!(store.backfill_folders().expect("first"), 1);
    let named = folder(&store, &id);
    assert_eq!(store.backfill_folders().expect("second"), 0);
    assert_eq!(folder(&store, &id), named);
}

#[test]
fn renaming_a_project_keeps_its_folder() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    let before = folder(&store, &id);
    assert!(store.rename_project(&id, "Something Else").expect("rename"));
    assert_eq!(folder(&store, &id), before);
}

#[test]
fn a_stored_folder_that_could_climb_out_is_refused() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    for climbing in ["..", "../x", "a/b", "/etc", "Demo", "a_b", ""] {
        set_folder(&store, climbing);
        assert!(
            matches!(
                ProjectHome::of(&store, dir.path(), &id),
                Err(HomeError::Forbidden)
            ),
            "{climbing:?} was accepted as a folder"
        );
    }
}

/// A synced row pointing at a folder no row owns, or another project's, would
/// have that folder's files served, written and wiped as this project's.
#[test]
fn a_stored_folder_not_named_after_its_own_id_is_refused() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    let own = folder(&store, &id);
    let tail = own.rsplit_once('-').expect("a suffix").1.to_owned();

    for foreign in [
        "demo-k387km".to_owned(),
        "orphan-abcdef".to_owned(),
        format!("demo-{}", &tail[1..]),
        format!("-{tail}"),
    ] {
        set_folder(&store, &foreign);
        assert!(
            matches!(
                ProjectHome::of(&store, dir.path(), &id),
                Err(HomeError::Forbidden)
            ),
            "{foreign:?} was accepted as {id}'s folder"
        );
    }
    set_folder(&store, &format!("renamed-since-{tail}"));
    assert!(ProjectHome::of(&store, dir.path(), &id).is_ok());
}

#[test]
fn a_folder_of_another_id_is_not_moved_into() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    let old = legacy(root, &id);
    set_folder(&store, "orphan-k387km");

    settle(&store, root).expect("start");

    assert!(
        old.join("sessions/conv_1.jsonl").exists(),
        "the files moved"
    );
    assert!(!projects_dir(root).join("orphan-k387km").exists());
}

/// The id reached `remove_dir_all` through `project_forget`; a `..` allowed
/// through is a delete of the workspace itself.
#[test]
fn a_project_id_that_could_climb_out_is_refused() {
    let (dir, store) = workspace();
    for climbing in ["..", "../..", "a/b", "/etc", "a\0b", ""] {
        assert!(
            matches!(
                ProjectHome::of(&store, dir.path(), climbing),
                Err(HomeError::Invalid)
            ),
            "{climbing:?} was accepted as a project id"
        );
    }
}

#[test]
fn an_unregistered_project_has_no_folder() {
    let (dir, store) = workspace();
    assert!(matches!(
        ProjectHome::of(&store, dir.path(), ID),
        Err(HomeError::NotFound)
    ));
}

#[test]
fn a_project_lands_under_the_workspace_in_its_named_folder() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    let home = ProjectHome::of(&store, dir.path(), &id).expect("home");
    let named = folder(&store, &id);
    assert!(home.dir().starts_with(dir.path()));
    assert!(home.dir().ends_with(format!("projects/{named}")));
    assert_eq!(home.relative(), format!("projects/{named}"));
}

#[test]
fn what_a_project_keeps_lives_in_its_folder() {
    let home = ProjectHome::at(Path::new("/home/me/.devpit"), ID, "demo-k387km").expect("home");
    assert_eq!(
        home.pasted(),
        Path::new("/home/me/.devpit/projects/demo-k387km/pasted")
    );
    assert_eq!(
        home.sessions(),
        Path::new("/home/me/.devpit/projects/demo-k387km/sessions")
    );
    assert_eq!(
        home.prime(),
        Path::new("/home/me/.devpit/projects/demo-k387km/prime.json")
    );
}

#[test]
fn a_folder_to_wipe_is_the_real_one_or_nothing() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    let home = ProjectHome::of(&store, dir.path(), &id).expect("home");
    assert_eq!(home.wipeable().expect("absent"), None);

    std::fs::create_dir_all(home.dir()).expect("made");

    let real = home.dir().canonicalize().expect("real");
    assert_eq!(home.wipeable().expect("present"), Some(real));
}

/// `remove_dir_all` would be handed the canonical path, so a link to the
/// workspace or to another project's folder is refused before anything goes.
#[cfg(unix)]
#[test]
fn a_folder_linked_anywhere_else_is_not_wiped() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    let home = ProjectHome::of(&store, root, &id).expect("home");
    let other = projects_dir(root).join("other-abcdef");
    std::fs::create_dir_all(&other).expect("another project's folder");

    for target in [root.to_path_buf(), other] {
        let _ = std::fs::remove_file(home.dir());
        std::os::unix::fs::symlink(&target, home.dir()).expect("link");
        assert!(
            matches!(home.wipeable(), Err(HomeError::Elsewhere)),
            "{target:?} was offered for a wipe"
        );
    }
}

#[test]
fn a_row_not_named_yet_still_finds_its_files_under_its_id() {
    let (dir, store) = workspace();
    let id = project(dir.path(), &store, "Demo Project");
    store
        .conn()
        .execute("UPDATE project SET folder = NULL", [])
        .expect("unnamed");
    let home = ProjectHome::of(&store, dir.path(), &id).expect("home");
    assert_eq!(home.dir(), projects_dir(dir.path()).join(&id));
}

/// Naming commits before the move; a move that failed must not have this
/// session read, and write, an empty named folder.
#[test]
fn a_named_folder_not_moved_yet_is_read_from_under_the_id() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    legacy(root, &id);

    let home = ProjectHome::of(&store, root, &id).expect("home");
    assert_eq!(
        std::fs::read_to_string(home.sessions().join("conv_1.jsonl")).expect("found"),
        "said\n"
    );

    let named = ProjectHome::at(root, &id, &folder(&store, &id)).expect("named");
    std::fs::create_dir_all(named.dir()).expect("named folder");
    assert_eq!(ProjectHome::of(&store, root, &id).expect("home"), named);
}

#[cfg(unix)]
#[test]
fn a_link_at_the_id_is_not_read_from() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    let elsewhere = legacy(root, "elsewhere");
    std::fs::create_dir_all(projects_dir(root)).expect("projects");
    std::os::unix::fs::symlink(&elsewhere, projects_dir(root).join(&id)).expect("link");

    let home = ProjectHome::of(&store, root, &id).expect("home");
    assert_eq!(home.folder(), folder(&store, &id));
}

#[test]
fn a_move_that_failed_once_still_happens_at_the_next_start() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    let old = legacy(root, &id);
    // What the session between the two starts writes.
    let sessions = ProjectHome::of(&store, root, &id).expect("home").sessions();
    std::fs::write(sessions.join("conv_2.jsonl"), "later\n").expect("a new conversation");

    settle(&store, root).expect("next start");

    assert!(!old.exists(), "the folder under the id is still there");
    let moved = ProjectHome::of(&store, root, &id).expect("home").sessions();
    assert!(moved.starts_with(projects_dir(root).join(folder(&store, &id))));
    for (file, said) in [("conv_1.jsonl", "said\n"), ("conv_2.jsonl", "later\n")] {
        assert_eq!(std::fs::read_to_string(moved.join(file)).expect(file), said);
    }
    assert!(store.notices(10).expect("notices").is_empty());
}

#[test]
fn settling_twice_leaves_one_folder_with_the_same_contents() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    store
        .conn()
        .execute("UPDATE project SET folder = NULL", [])
        .expect("a row from before migration 12");
    let old = legacy(root, &id);

    settle(&store, root).expect("first start");
    settle(&store, root).expect("second start");

    assert!(!old.exists(), "the folder under the id is still there");
    let sessions = ProjectHome::of(&store, root, &id).expect("home").sessions();
    assert_eq!(
        std::fs::read_to_string(sessions.join("conv_1.jsonl")).expect("moved"),
        "said\n"
    );
    let folders = std::fs::read_dir(projects_dir(root)).expect("list").count();
    assert_eq!(folders, 1);
    assert!(store.notices(10).expect("notices").is_empty());
}

#[test]
fn a_folder_already_at_the_destination_is_not_overwritten_and_says_so() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    let old = legacy(root, &id);
    let new = ProjectHome::at(root, &id, &folder(&store, &id))
        .expect("home")
        .dir();
    std::fs::create_dir_all(&new).expect("destination");
    std::fs::write(new.join("mine.txt"), "kept").expect("a file already there");

    settle(&store, root).expect("first start");
    settle(&store, root).expect("second start");

    assert_eq!(
        std::fs::read_to_string(old.join("sessions/conv_1.jsonl")).expect("left"),
        "said\n"
    );
    assert_eq!(
        std::fs::read_to_string(new.join("mine.txt")).expect("kept"),
        "kept"
    );
    assert!(!new.join("sessions").exists(), "the two were merged");
    let told: Vec<_> = store
        .notices(10)
        .expect("notices")
        .into_iter()
        .filter(|row| row.kind == FOLDER_NOTICE)
        .collect();
    assert_eq!(told.len(), 1, "told once, not once per start");
    assert_eq!(told[0].project_id.as_deref(), Some(id.as_str()));
    // The tempdir's own name is part of every absolute path under it.
    let detail = told[0].detail.as_deref().expect("a detail");
    let spelled = root.file_name().expect("a name").to_string_lossy();
    assert!(
        !detail.contains(&*spelled),
        "a path reached the screen: {detail}"
    );
}

#[test]
fn pins_follow_the_folder_they_were_in() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    let old = legacy(root, &id);
    store.ensure_board(&id).expect("board");
    let column = store.columns(&id).expect("columns")[0].id.clone();
    let card = store.create_card(&id, &column, "a card", "").expect("card");
    let elsewhere = root.join("checkouts/Demo Project/README.md");
    store
        .attach(
            &card,
            &old.join("sessions/conv_1.jsonl").display().to_string(),
            "t",
            None,
        )
        .expect("pin");
    store
        .attach(&card, &elsewhere.display().to_string(), "readme", None)
        .expect("pin");

    settle(&store, root).expect("start");

    let pinned: Vec<String> = store
        .attachments(&card)
        .expect("pins")
        .into_iter()
        .map(|pin| pin.path)
        .collect();
    let moved = ProjectHome::of(&store, root, &id)
        .expect("home")
        .sessions()
        .join("conv_1.jsonl");
    assert!(pinned.contains(&moved.display().to_string()), "{pinned:?}");
    assert!(
        pinned.contains(&elsewhere.display().to_string()),
        "{pinned:?}"
    );
}

/// A transcript line is never rewritten, so the path in it stays the old one.
#[test]
fn a_picture_pasted_before_the_move_is_found_after_it() {
    let (dir, store) = workspace();
    let root = dir.path();
    let id = project(root, &store, "Demo Project");
    let old = legacy(root, &id);
    std::fs::create_dir_all(old.join("pasted")).expect("pasted");
    let written = old.join("pasted/pasted-1.png");
    std::fs::write(&written, "png").expect("picture");

    settle(&store, root).expect("start");

    let found = moved(&store, root, &written).expect("found in the named folder");
    assert_eq!(
        found,
        ProjectHome::of(&store, root, &id)
            .expect("home")
            .pasted()
            .join("pasted-1.png")
    );
    assert!(
        moved(&store, root, &found).is_none(),
        "a path that exists is kept"
    );
    let climbing = old.join("pasted/../pasted/pasted-1.png");
    assert!(
        moved(&store, root, &climbing).is_none(),
        "a path with `..` in it was looked for"
    );
}

#[test]
fn two_projects_never_share_a_folder() {
    let (dir, store) = workspace();
    let first = project(dir.path(), &store, "Demo Project");
    let second = project(dir.path(), &store, "Other");
    let taken = folder(&store, &first);
    let shared = store.conn().execute(
        "UPDATE project SET folder = ?1 WHERE id = ?2",
        [taken.as_str(), second.as_str()],
    );
    assert!(shared.is_err(), "two projects were given one folder");
}

#[test]
fn a_plugin_folder_is_only_for_an_id_a_manifest_could_carry() {
    let home = ProjectHome::at(Path::new("/nowhere"), ID, "demo-k387km").expect("a folder");
    let long = "a".repeat(33);
    for id in [
        "..",
        "a/b",
        "../x",
        "Excalidraw",
        "x",
        "-x",
        "",
        "a_b",
        &long,
    ] {
        assert!(
            matches!(home.plugin_data(id), Err(HomeError::PluginId)),
            "{id:?} was given a folder"
        );
    }
    assert!(home.plugin_data("excalidraw").is_ok());
}

#[test]
fn a_data_name_windows_or_a_listing_would_misread_is_refused() {
    // 190 + 11 is one byte past the 200 a name may have.
    let long = format!("{}.excalidraw", "a".repeat(190));
    for name in [
        "con.excalidraw",
        "NUL.excalidraw",
        "Com1.excalidraw",
        "lpt9.old.excalidraw",
        "aux .excalidraw",
        "flow.excalidraw.",
        "flow.excalidraw ",
        "a<b.excalidraw",
        "a>b.excalidraw",
        "a\"b.excalidraw",
        "a|b.excalidraw",
        "a?b.excalidraw",
        "a*b.excalidraw",
        "a\u{7}b.excalidraw",
        "a\u{202E}b.excalidraw",
        "a\u{2067}b.excalidraw",
        &long,
    ] {
        assert!(!plain_data_name(name), "{name:?} was accepted");
    }
}

#[test]
fn a_data_name_that_only_starts_like_a_device_is_kept() {
    // 189 + 11 is exactly the 200 a name may have.
    let longest = format!("{}.excalidraw", "a".repeat(189));
    for name in [
        "console.excalidraw",
        "com10.excalidraw",
        "nullish.excalidraw",
        "Überblick.excalidraw",
        "flow (2).excalidraw",
        &longest,
    ] {
        assert!(plain_data_name(name), "{name:?} was refused");
    }
}

const PLUGIN: &str = "excalidraw";

/// A project with the plugin installed and one file in its data folder.
fn installed(root: &Path, store: &Store) -> (String, ProjectHome) {
    let id = project(root, store, "demo");
    store.install_plugin(&id, PLUGIN).expect("install");
    let home = ProjectHome::of(store, root, &id).expect("home");
    let own = home.dir().join(home.plugin_data(PLUGIN).expect("an id"));
    std::fs::create_dir_all(&own).expect("data");
    std::fs::write(own.join("flow.excalidraw"), "scene").expect("a file");
    (id, home)
}

fn data_of(home: &ProjectHome) -> PathBuf {
    home.dir().join("data")
}

#[test]
fn an_uninstall_that_keeps_the_data_touches_no_file() {
    let (dir, store) = workspace();
    let (id, home) = installed(dir.path(), &store);

    let removed = uninstall_plugin(&store, dir.path(), &id, PLUGIN, false).expect("uninstall");

    assert_eq!(removed, 0);
    let file = data_of(&home).join(PLUGIN).join("flow.excalidraw");
    assert_eq!(std::fs::read_to_string(file).expect("kept"), "scene");
    assert!(store.installed_plugins(&id).expect("read").is_empty());
}

#[test]
fn an_uninstall_deletes_only_its_plugins_folder_and_counts_its_files() {
    let (dir, store) = workspace();
    let (id, home) = installed(dir.path(), &store);
    let own = data_of(&home).join(PLUGIN);
    std::fs::create_dir_all(own.join("nested")).expect("nested");
    std::fs::write(own.join("nested/old.excalidraw"), "scene").expect("a nested file");
    let sibling = data_of(&home).join("other-plugin");
    std::fs::create_dir_all(&sibling).expect("sibling");
    std::fs::write(sibling.join("keep.txt"), "theirs").expect("a sibling's file");
    std::fs::write(home.prime(), "{}").expect("a project file");

    let removed = uninstall_plugin(&store, dir.path(), &id, PLUGIN, true).expect("uninstall");

    assert_eq!(removed, 2, "flow.excalidraw and nested/old.excalidraw");
    assert!(own.symlink_metadata().is_err());
    let theirs = std::fs::read_to_string(sibling.join("keep.txt")).expect("kept");
    assert_eq!(theirs, "theirs");
    assert!(home.prime().is_file());
    assert!(store.installed_plugins(&id).expect("read").is_empty());
}

/// A delete that fails partway must not leave a plugin marked uninstalled.
#[cfg(unix)]
#[test]
fn a_delete_that_fails_leaves_the_plugin_installed() {
    use std::os::unix::fs::PermissionsExt;

    let (dir, store) = workspace();
    let (id, home) = installed(dir.path(), &store);
    let locked = data_of(&home).join(PLUGIN).join("locked");
    std::fs::create_dir_all(&locked).expect("locked");
    std::fs::write(locked.join("held.excalidraw"), "scene").expect("a held file");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).expect("lock");
    // Root ignores the mode, so there is no failure to observe there.
    if std::fs::write(locked.join("probe"), "").is_ok() {
        return;
    }

    let refused = uninstall_plugin(&store, dir.path(), &id, PLUGIN, true);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).expect("unlock");

    assert!(matches!(refused, Err(HomeError::Removal(_))), "{refused:?}");
    assert_eq!(
        store.installed_plugins(&id).expect("read"),
        vec![PLUGIN.to_owned()]
    );
}

/// The link goes with the folder; the file it points at is not the plugin's.
#[cfg(unix)]
#[test]
fn a_link_inside_the_folder_is_not_counted_and_its_target_stays() {
    let (dir, store) = workspace();
    let (id, home) = installed(dir.path(), &store);
    let outside = dir.path().join("outside.excalidraw");
    std::fs::write(&outside, "not the plugin's").expect("outside");
    let link = data_of(&home).join(PLUGIN).join("link.excalidraw");
    std::os::unix::fs::symlink(&outside, link).expect("link");

    let removed = uninstall_plugin(&store, dir.path(), &id, PLUGIN, true).expect("uninstall");

    assert_eq!(removed, 1, "only flow.excalidraw is a file");
    let kept = std::fs::read_to_string(&outside).expect("kept");
    assert_eq!(kept, "not the plugin's");
}

/// A refused delete changes nothing: not the target, not the install.
#[cfg(unix)]
fn refused_and_untouched(dir: &Path, store: &Store, id: &str, target: &Path) {
    let err = uninstall_plugin(store, dir, id, PLUGIN, true).expect_err("refused");
    assert!(matches!(err, HomeError::PluginElsewhere), "{err}");
    let kept = std::fs::read_to_string(target).expect("kept");
    assert_eq!(kept, "not the plugin's");
    assert_eq!(store.installed_plugins(id).expect("read"), [PLUGIN]);
}

#[cfg(unix)]
#[test]
fn a_plugin_folder_that_is_a_link_is_refused_and_its_target_untouched() {
    let (dir, store) = workspace();
    let (id, home) = installed(dir.path(), &store);
    let own = data_of(&home).join(PLUGIN);
    std::fs::remove_dir_all(&own).expect("room for the link");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("elsewhere");
    let target = elsewhere.join("mine.excalidraw");
    std::fs::write(&target, "not the plugin's").expect("a file");
    std::os::unix::fs::symlink(&elsewhere, &own).expect("link");

    refused_and_untouched(dir.path(), &store, &id, &target);
    assert!(own
        .symlink_metadata()
        .expect("the link stayed")
        .is_symlink());
}

#[cfg(unix)]
#[test]
fn a_data_folder_that_is_a_link_is_refused_and_its_target_untouched() {
    let (dir, store) = workspace();
    let (id, home) = installed(dir.path(), &store);
    std::fs::remove_dir_all(data_of(&home)).expect("room for the link");
    let elsewhere = dir.path().join("elsewhere");
    std::fs::create_dir_all(elsewhere.join(PLUGIN)).expect("elsewhere");
    let target = elsewhere.join(PLUGIN).join("mine.excalidraw");
    std::fs::write(&target, "not the plugin's").expect("a file");
    std::os::unix::fs::symlink(&elsewhere, data_of(&home)).expect("link");

    refused_and_untouched(dir.path(), &store, &id, &target);
}

#[test]
fn an_unregistered_project_is_not_found_to_uninstall_from() {
    let (dir, store) = workspace();
    let err = uninstall_plugin(&store, dir.path(), "prj_missing", PLUGIN, true);
    assert!(matches!(err, Err(HomeError::NotFound)), "{err:?}");
}
