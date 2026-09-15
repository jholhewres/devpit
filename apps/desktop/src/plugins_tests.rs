use std::path::PathBuf;

use devpit_core::home::ProjectHome;

use super::*;

const PLUGIN: &str = "excalidraw";

fn project() -> (tempfile::TempDir, Store, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let checkout = dir.path().join("checkout");
    std::fs::create_dir_all(&checkout).expect("checkout");
    let id = store.add_project(&checkout, None).expect("project");
    (dir, store, id)
}

fn state<'a>(plugins: &'a [PluginState], plugin_id: &str) -> &'a PluginState {
    plugins
        .iter()
        .find(|one| one.manifest.id == plugin_id)
        .expect("in the catalogue")
}

/// The plugin's data folder, made, holding `files`.
fn data_with(root: &Path, store: &Store, id: &str, files: &[&str]) -> PathBuf {
    let home = ProjectHome::of(store, root, id).expect("home");
    let data = home.dir().join(home.plugin_data(PLUGIN).expect("an id"));
    std::fs::create_dir_all(&data).expect("data");
    for file in files {
        std::fs::write(data.join(file), "scene").expect("a file");
    }
    data
}

#[test]
fn the_catalogue_is_listed_uninstalled_until_installed() {
    let (_dir, store, id) = project();
    let before = listed(&store, &id).expect("list");
    let one = state(&before.plugins, PLUGIN);
    assert_eq!((one.installed, one.enabled), (false, false));

    let answered = install(&store, &id, PLUGIN).expect("install");
    let one = state(&answered.plugins, PLUGIN);
    assert_eq!((one.installed, one.enabled), (true, true));
    assert_eq!(listed(&store, &id).expect("list"), answered);

    let again = install(&store, &id, PLUGIN).expect("installing twice is fine");
    assert_eq!(again, answered);
}

#[test]
fn turning_on_a_plugin_that_is_not_installed_is_forbidden() {
    let (_dir, store, id) = project();

    let err = set_enabled(&store, &id, PLUGIN, true).expect_err("refused");

    assert_eq!(err.code, ErrorCode::Forbidden, "{}", err.message);
    assert!(store.enabled_plugins(&id).expect("read").is_empty());
    install(&store, &id, PLUGIN).expect("install");
    let off = set_enabled(&store, &id, PLUGIN, false).expect("installed, so it turns");
    let one = state(&off.plugins, PLUGIN);
    assert_eq!((one.installed, one.enabled), (true, false));
}

#[test]
fn a_plugin_this_build_does_not_ship_is_not_found() {
    let (dir, store, id) = project();
    let answers = [
        set_enabled(&store, &id, "nope", true).map(|_| ()),
        install(&store, &id, "nope").map(|_| ()),
        uninstall(&store, dir.path(), &id, "nope", true).map(|_| ()),
    ];
    for answer in answers {
        assert_eq!(answer.expect_err("refused").code, ErrorCode::NotFound);
    }
    assert!(store.installed_plugins(&id).expect("read").is_empty());
}

#[test]
fn a_project_that_is_not_registered_is_not_found() {
    let (dir, store, _) = project();
    let answers = [
        listed(&store, "prj_missing").map(|_| ()),
        install(&store, "prj_missing", PLUGIN).map(|_| ()),
        uninstall(&store, dir.path(), "prj_missing", PLUGIN, true).map(|_| ()),
    ];
    for answer in answers {
        let err = answer.expect_err("refused");
        assert_eq!(err.code, ErrorCode::NotFound, "{}", err.message);
    }
}

#[test]
fn uninstalling_answers_with_the_catalogue_and_how_many_files_went() {
    let (dir, store, id) = project();
    let root = dir.path();
    install(&store, &id, PLUGIN).expect("install");
    let data = data_with(root, &store, &id, &["a.excalidraw", "b.excalidraw"]);

    let kept = uninstall(&store, root, &id, PLUGIN, false).expect("uninstall");
    let one = state(&kept.plugins, PLUGIN);
    assert_eq!((one.installed, one.enabled), (false, false));
    assert_eq!(kept.removed_files, 0);
    assert!(data.join("a.excalidraw").is_file());

    install(&store, &id, PLUGIN).expect("reinstall");
    let wiped = uninstall(&store, root, &id, PLUGIN, true).expect("uninstall");
    assert_eq!(wiped.removed_files, 2, "the two files written above");
    assert!(!data.exists());
}

/// Checked before the row changes, so a refused delete leaves it installed.
#[cfg(unix)]
#[test]
fn a_plugin_folder_linked_out_is_forbidden_to_delete_and_stays_installed() {
    let (dir, store, id) = project();
    let root = dir.path();
    install(&store, &id, PLUGIN).expect("install");
    let data = data_with(root, &store, &id, &[]);
    std::fs::remove_dir(&data).expect("room for the link");
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("elsewhere");
    std::fs::write(elsewhere.join("mine.excalidraw"), "not the plugin's").expect("a file");
    std::os::unix::fs::symlink(&elsewhere, &data).expect("link");

    let err = uninstall(&store, root, &id, PLUGIN, true).expect_err("refused");

    assert_eq!(err.code, ErrorCode::Forbidden, "{}", err.message);
    let kept = std::fs::read_to_string(elsewhere.join("mine.excalidraw")).expect("kept");
    assert_eq!(kept, "not the plugin's");
    assert_eq!(store.installed_plugins(&id).expect("read"), [PLUGIN]);
}
