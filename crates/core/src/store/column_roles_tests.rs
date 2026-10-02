use super::{role_from_name, role_of};
use crate::store::Store;

#[test]
fn a_column_is_read_by_its_name() {
    for (name, role) in [
        ("inbox", Some("backlog")),
        ("To Do", Some("backlog")),
        ("refine", Some("backlog")),
        ("doing", Some("doing")),
        ("In progress", Some("doing")),
        ("Em andamento", Some("doing")),
        ("check", Some("check")),
        ("Code review", Some("check")),
        ("ship", Some("done")),
        ("Concluído", Some("done")),
        ("Ideias malucas", None),
    ] {
        assert_eq!(role_from_name(name), role, "{name}");
    }
}

#[test]
fn the_persons_choice_wins_over_the_name() {
    assert_eq!(role_of(Some("check"), "doing"), Some("check"));
    assert_eq!(role_of(None, "doing"), Some("doing"));
    assert_eq!(
        role_of(Some("nonsense"), "doing"),
        None,
        "an unknown role is no role"
    );
}

#[test]
fn a_role_is_kept_and_let_go() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("root");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns").remove(0);
    assert!(store
        .set_column_role(&column.id, Some("doing"))
        .expect("set"));
    assert_eq!(
        store.columns(&project).expect("columns")[0].role.as_deref(),
        Some("doing")
    );
    assert!(!store
        .set_column_role(&column.id, Some("nonsense"))
        .expect("refused"));
    assert!(store.set_column_role(&column.id, None).expect("cleared"));
    assert_eq!(store.columns(&project).expect("columns")[0].role, None);
}
