//! Plugins turned on per project, and drawings leaving the database as files.

use std::path::PathBuf;
use std::time::Duration;

use rusqlite::params;

use super::*;
use crate::store::migrations;

const ID: &str = "prj_01M24GHNGDMZCXRFWEDVK387KM";
const FOLDER: &str = "demo-k387km";
const SCENE: &str = r#"{"type":"excalidraw","version":2,"elements":[]}"#;

/// A database as an older build left it: one project, and its drawings.
fn older(root: &Path, version: i64, folder: Option<&str>, names: &[&str]) {
    let conn = Connection::open(root.join("state.db")).expect("open raw");
    conn.pragma_update(None, "foreign_keys", "ON").expect("fk");
    migrations::run_up_to(&conn, version).expect("migrate");
    conn.execute_batch(
        "INSERT INTO trust_workspace (id, slug, label, color, vault_namespace, is_default, created_at) \
         VALUES ('tw_1', 'p', 'P', '#fff', 'p', 1, 0);",
    )
    .expect("workspace");
    conn.execute(
        "INSERT INTO project (id, trust_workspace_id, name, root_path, created_at) \
         VALUES (?1, 'tw_1', 'demo', '/tmp/demo', 0)",
        [ID],
    )
    .expect("project");
    if let Some(folder) = folder {
        conn.execute("UPDATE project SET folder = ?1", [folder])
            .expect("folder");
    }
    for (n, name) in names.iter().enumerate() {
        conn.execute(
            "INSERT INTO drawing (id, project_id, name, scene, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, 0, 0)",
            params![format!("drw_{n}"), ID, name, SCENE],
        )
        .expect("drawing");
    }
}

/// Where the Excalidraw plugin keeps the project's files.
fn drawings(root: &Path, folder: Option<&str>) -> PathBuf {
    let home = ProjectHome::named(root, ID, folder.map(str::to_owned)).expect("a folder");
    home.dir()
        .join(home.plugin_data(DRAWINGS_PLUGIN).expect("an id"))
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |row| row.get(0))
        .expect("version")
}

/// A file where a folder the export needs goes: a mode bit would not stop root.
fn blocked(path: &Path) {
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("parent");
    std::fs::write(path, "").expect("blocker");
}

/// Opens the store on another thread: an export that opened a FIFO would
/// never come back, and a test that hangs says less than one that fails.
fn opened_in_time(root: &Path) -> Result<Store, StoreError> {
    let db = root.join("state.db");
    let (sent, answer) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = sent.send(Store::open(&db));
    });
    answer
        .recv_timeout(Duration::from_secs(10))
        .expect("the export waited on a FIFO")
}

#[cfg(unix)]
fn fifo(path: &Path) {
    let made = std::process::Command::new("mkfifo")
        .arg(path)
        .status()
        .expect("mkfifo to run");
    assert!(made.success(), "mkfifo made no pipe");
}

#[cfg(unix)]
fn is_fifo(path: &Path) -> bool {
    use std::os::unix::fs::FileTypeExt;
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_fifo())
}

#[cfg(unix)]
fn is_link(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
}

#[test]
fn a_drawing_is_a_file_before_its_table_goes() {
    let dir = tempfile::tempdir().expect("tempdir");
    older(dir.path(), 12, Some(FOLDER), &["flow"]);

    let store = Store::open(&dir.path().join("state.db")).expect("migrate");

    let file = drawings(dir.path(), Some(FOLDER)).join("flow.excalidraw");
    assert_eq!(std::fs::read_to_string(file).expect("exported"), SCENE);
    assert_eq!(user_version(store.conn()), migrations::latest());
    let tables: i64 = store
        .conn()
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = 'drawing'",
            [],
            |row| row.get(0),
        )
        .expect("catalog");
    assert_eq!(tables, 0);
}

/// A planted file must not keep the app from opening, and the scene is not
/// lost for it: it lands beside the database, and the bell says where.
#[test]
fn a_drawing_its_project_folder_will_not_take_is_kept_and_said_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    older(root, 12, Some(FOLDER), &["flow"]);
    blocked(drawings(root, Some(FOLDER)).parent().expect("data"));

    let store = Store::open(&root.join("state.db")).expect("the app still opens");

    let kept = root.join(KEPT_DRAWINGS).join("drw_0.excalidraw");
    assert_eq!(std::fs::read_to_string(kept).expect("kept"), SCENE);
    assert_eq!(user_version(store.conn()), migrations::latest());
    let told: Vec<_> = store
        .notices(10)
        .expect("notices")
        .into_iter()
        .filter(|row| row.kind == DRAWING_NOTICE)
        .collect();
    assert_eq!(told.len(), 1);
    assert_eq!(told[0].project_id.as_deref(), Some(ID));
    let detail = told[0].detail.as_deref().expect("says where");
    assert!(detail.contains("drawings/drw_0.excalidraw"), "{detail}");
    let spelled = root.file_name().expect("a name").to_string_lossy();
    assert!(
        !detail.contains(&*spelled),
        "a path reached the screen: {detail}"
    );
}

/// Dropping the table would lose a scene nothing could take, so the
/// migration waits for the next start instead.
#[test]
fn a_drawing_nothing_will_take_keeps_the_database_as_it_was() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    older(root, 12, Some(FOLDER), &["flow"]);
    blocked(drawings(root, Some(FOLDER)).parent().expect("data"));
    blocked(&root.join(KEPT_DRAWINGS));

    let err = Store::open(&root.join("state.db"))
        .err()
        .expect("the open was refused");
    assert!(matches!(err, StoreError::Export { .. }), "{err}");
    let spelled = root.file_name().expect("a name").to_string_lossy();
    assert!(
        !err.to_string().contains(&*spelled),
        "a path reached the screen: {err}"
    );

    let conn = Connection::open(root.join("state.db")).expect("open raw");
    assert_eq!(user_version(&conn), 12);
    let scene: String = conn
        .query_row("SELECT scene FROM drawing WHERE name = 'flow'", [], |row| {
            row.get(0)
        })
        .expect("the row stayed");
    assert_eq!(scene, SCENE);
}

#[test]
fn a_project_no_build_has_named_keeps_its_drawings_under_its_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    older(dir.path(), 11, None, &["flow"]);

    Store::open(&dir.path().join("state.db")).expect("migrate");

    let file = drawings(dir.path(), None).join("flow.excalidraw");
    assert_eq!(std::fs::read_to_string(file).expect("exported"), SCENE);
}

#[test]
fn a_drawing_named_like_a_path_is_saved_under_its_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    older(dir.path(), 12, Some(FOLDER), &["../../../escaped"]);

    Store::open(&dir.path().join("state.db")).expect("migrate");

    let folder = drawings(dir.path(), Some(FOLDER));
    assert_eq!(
        std::fs::read_to_string(folder.join("drw_0.excalidraw")).expect("exported"),
        SCENE
    );
    assert!(!folder.join("../../../escaped.excalidraw").exists());
    assert_eq!(std::fs::read_dir(&folder).expect("listing").count(), 1);
}

/// Both would write fine on Linux: one the listing never shows, one Windows
/// opens as a device.
#[test]
fn a_drawing_named_no_plugin_file_name_is_saved_under_its_id() {
    let dir = tempfile::tempdir().expect("tempdir");
    older(dir.path(), 12, Some(FOLDER), &[".hidden", "nul"]);

    Store::open(&dir.path().join("state.db")).expect("migrate");

    let folder = drawings(dir.path(), Some(FOLDER));
    let mut names: Vec<String> = std::fs::read_dir(&folder)
        .expect("listing")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, ["drw_0.excalidraw", "drw_1.excalidraw"]);
}

#[test]
fn a_drawing_never_writes_over_a_different_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    older(dir.path(), 12, Some(FOLDER), &["flow"]);
    let folder = drawings(dir.path(), Some(FOLDER));
    std::fs::create_dir_all(&folder).expect("folder");
    std::fs::write(folder.join("flow.excalidraw"), "mine").expect("a file already there");

    Store::open(&dir.path().join("state.db")).expect("migrate");

    let kept = std::fs::read_to_string(folder.join("flow.excalidraw")).expect("kept");
    assert_eq!(kept, "mine");
    let moved = std::fs::read_to_string(folder.join("drw_0.excalidraw")).expect("exported");
    assert_eq!(moved, SCENE);
}

/// `O_CREAT` follows a dangling link and makes its target: a desktop
/// autostart entry, say.
#[cfg(unix)]
#[test]
fn a_dangling_link_at_the_id_name_is_not_written_through() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    older(root, 12, Some(FOLDER), &["flow"]);
    let folder = drawings(root, Some(FOLDER));
    std::fs::create_dir_all(&folder).expect("folder");
    std::fs::write(folder.join("flow.excalidraw"), "mine").expect("so the id name is chosen");
    let target = root.join("autostart.desktop");
    std::os::unix::fs::symlink(&target, folder.join("drw_0.excalidraw")).expect("link");

    Store::open(&root.join("state.db")).expect("migrate");

    assert!(
        target.symlink_metadata().is_err(),
        "the export wrote through the link"
    );
    assert!(is_link(&folder.join("drw_0.excalidraw")));
    let kept = root.join(KEPT_DRAWINGS).join("drw_0.excalidraw");
    assert_eq!(std::fs::read_to_string(kept).expect("kept"), SCENE);
}

/// The link points at a file holding this very scene: followed, it would pass
/// for an export already done, and the drawing would live outside its folder.
#[cfg(unix)]
#[test]
fn a_fifo_or_a_link_at_the_name_is_neither_waited_on_nor_followed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    older(root, 12, Some(FOLDER), &["flow", "chart"]);
    let folder = drawings(root, Some(FOLDER));
    std::fs::create_dir_all(&folder).expect("folder");
    fifo(&folder.join("flow.excalidraw"));
    let outside = root.join("outside.excalidraw");
    std::fs::write(&outside, SCENE).expect("outside");
    std::os::unix::fs::symlink(&outside, folder.join("chart.excalidraw")).expect("link");

    opened_in_time(root).expect("migrate");

    for exported in ["drw_0.excalidraw", "drw_1.excalidraw"] {
        let file = std::fs::read_to_string(folder.join(exported)).expect(exported);
        assert_eq!(file, SCENE);
    }
    assert!(is_fifo(&folder.join("flow.excalidraw")));
    assert!(is_link(&folder.join("chart.excalidraw")));
}

/// A FIFO where the name climbs to: reading it, as a comparison would, never
/// returns.
#[cfg(unix)]
#[test]
fn a_drawing_named_to_climb_out_reads_nothing_outside_its_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    older(root, 12, Some(FOLDER), &["../../x"]);
    let folder = drawings(root, Some(FOLDER));
    std::fs::create_dir_all(&folder).expect("folder");
    let climbed = folder.join("../../x.excalidraw");
    fifo(&climbed);

    opened_in_time(root).expect("migrate");

    let file = std::fs::read_to_string(folder.join("drw_0.excalidraw")).expect("exported");
    assert_eq!(file, SCENE);
    assert!(is_fifo(&climbed));
}

/// `create_dir_all` walks through a link, so every export would land wherever
/// a planted `data` points.
#[cfg(unix)]
#[test]
fn a_data_folder_linked_out_is_not_written_into() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    older(root, 12, Some(FOLDER), &["flow"]);
    let data = drawings(root, Some(FOLDER))
        .parent()
        .expect("data")
        .to_path_buf();
    std::fs::create_dir_all(data.parent().expect("project")).expect("project folder");
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).expect("elsewhere");
    std::os::unix::fs::symlink(&elsewhere, &data).expect("link");

    Store::open(&root.join("state.db")).expect("migrate");

    assert_eq!(std::fs::read_dir(&elsewhere).expect("listing").count(), 0);
    let kept = root.join(KEPT_DRAWINGS).join("drw_0.excalidraw");
    assert_eq!(std::fs::read_to_string(kept).expect("kept"), SCENE);
}

fn project(dir: &Path, store: &Store) -> String {
    let checkout = dir.join("checkout");
    std::fs::create_dir_all(&checkout).expect("checkout");
    store.add_project(&checkout, None).expect("project")
}

fn opened(dir: &Path) -> (Store, String) {
    let store = Store::open(&dir.join("state.db")).expect("open");
    let id = project(dir, &store);
    (store, id)
}

fn revision(store: &Store, id: &str) -> i64 {
    store
        .conn()
        .query_row(
            "SELECT revision FROM project_plugin WHERE project_id = ?1 AND plugin_id = ?2",
            params![id, DRAWINGS_PLUGIN],
            |row| row.get(0),
        )
        .expect("a row")
}

#[test]
fn a_plugin_is_off_until_it_is_installed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = opened(dir.path());
    assert!(store.installed_plugins(&id).expect("read").is_empty());
    assert!(store.enabled_plugins(&id).expect("read").is_empty());

    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("install");
    assert_eq!(
        store.installed_plugins(&id).expect("read"),
        [DRAWINGS_PLUGIN]
    );
    assert_eq!(store.enabled_plugins(&id).expect("read"), [DRAWINGS_PLUGIN]);

    let turned = store.set_plugin_enabled(&id, DRAWINGS_PLUGIN, false);
    assert!(turned.expect("off"));
    assert!(store.enabled_plugins(&id).expect("read").is_empty());
    assert_eq!(
        store.installed_plugins(&id).expect("read"),
        [DRAWINGS_PLUGIN]
    );
}

#[test]
fn a_plugin_that_is_not_installed_cannot_be_turned_on() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = opened(dir.path());
    let never = store.set_plugin_enabled(&id, DRAWINGS_PLUGIN, true);
    assert!(!never.expect("asked"));

    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("install");
    store
        .uninstall_plugin(&id, DRAWINGS_PLUGIN)
        .expect("uninstall");
    let after = store.set_plugin_enabled(&id, DRAWINGS_PLUGIN, true);

    assert!(!after.expect("asked"));
    assert!(store.enabled_plugins(&id).expect("read").is_empty());
}

/// What a row synced from elsewhere could say: on, but never installed here.
#[test]
fn a_row_that_is_on_but_not_installed_is_not_enabled() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = opened(dir.path());
    store
        .conn()
        .execute(
            "INSERT INTO project_plugin (project_id, plugin_id, installed, enabled, updated_at) \
             VALUES (?1, ?2, 0, 1, 0)",
            params![id, DRAWINGS_PLUGIN],
        )
        .expect("a synced row");

    assert!(store.enabled_plugins(&id).expect("read").is_empty());
}

#[test]
fn an_uninstall_keeps_the_row_with_both_off() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = opened(dir.path());
    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("install");

    store
        .uninstall_plugin(&id, DRAWINGS_PLUGIN)
        .expect("uninstall");

    let flags: (bool, bool) = store
        .conn()
        .query_row(
            "SELECT installed, enabled FROM project_plugin WHERE project_id = ?1",
            [&id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("the row stayed");
    assert_eq!(flags, (false, false));
}

/// The revision is what sync compares, so a change it misses never leaves.
#[test]
fn installing_and_uninstalling_each_move_the_revision_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = opened(dir.path());

    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("install");
    let installed = revision(&store, &id);
    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("again");
    assert_eq!(revision(&store, &id), installed, "nothing changed");

    store
        .uninstall_plugin(&id, DRAWINGS_PLUGIN)
        .expect("uninstall");
    let uninstalled = revision(&store, &id);
    assert!(uninstalled > installed);
    store.uninstall_plugin(&id, DRAWINGS_PLUGIN).expect("again");
    assert_eq!(revision(&store, &id), uninstalled, "nothing changed");

    store
        .install_plugin(&id, DRAWINGS_PLUGIN)
        .expect("reinstall");
    assert!(revision(&store, &id) > uninstalled);
}

#[test]
fn installing_again_leaves_a_plugin_that_was_turned_off_off() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = opened(dir.path());
    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("install");
    let turned = store.set_plugin_enabled(&id, DRAWINGS_PLUGIN, false);
    assert!(turned.expect("off"));

    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("again");

    assert!(store.enabled_plugins(&id).expect("read").is_empty());
}

#[test]
fn a_projects_plugins_go_with_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, id) = opened(dir.path());
    store.install_plugin(&id, DRAWINGS_PLUGIN).expect("install");

    assert!(store.erase_project(&id).expect("erase"));

    let rows: i64 = store
        .conn()
        .query_row("SELECT count(*) FROM project_plugin", [], |row| row.get(0))
        .expect("count");
    assert_eq!(rows, 0);
}
