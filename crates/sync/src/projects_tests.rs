use super::*;

const ORIGIN: &str = "git@github.com:someone/thing.git";

fn store() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    (dir, store)
}

fn record_of(store: &Store, project: &str) -> String {
    let listed = store.synced_projects().expect("list");
    let found = listed.iter().find(|one| one.id == project).expect("listed");
    identity(found.origin_hash.as_deref().expect("an origin"))
}

#[test]
fn a_clone_of_the_same_repository_is_the_same_record_and_a_folder_without_one_is_none() {
    let (dir, store) = store();
    let first = store
        .add_project(&dir.path().join("a"), Some(ORIGIN))
        .expect("add");
    store
        .add_project(
            &dir.path().join("b"),
            Some("https://github.com/someone/thing"),
        )
        .expect("add");
    store.add_project(&dir.path().join("c"), None).expect("add");
    let projects = store.synced_projects().expect("list");
    let carried = carriers(&projects);
    assert_eq!(carried.len(), 1, "one repository, one record");
    let (record, carrier) = carried.iter().next().expect("one");
    assert!(record.starts_with("origin:"));
    let oldest = projects
        .iter()
        .filter(|one| one.origin_hash.is_some())
        .min_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)))
        .expect("a clone");
    assert_eq!(carrier.id, oldest.id);
    assert!(record_of(&store, &first).starts_with("origin:"));
}

#[test]
fn a_value_from_elsewhere_is_held_to_the_edit_dialogs_rules() {
    assert!(fits(NAME, Some("API")));
    assert!(!fits(NAME, Some("  ")));
    assert!(!fits(NAME, None));
    assert!(!fits(NAME, Some(&"n".repeat(121))));
    assert!(fits(COLOR, Some("#a1b2c3")));
    assert!(fits(COLOR, None));
    assert!(!fits(COLOR, Some("red; background: url(x)")));
    assert!(fits(ICON, Some("rocket")));
    assert!(!fits(ICON, Some(&"i".repeat(33))));
    assert!(!fits(GROUP, Some("")));
    assert!(fits(DECISIONS_NEVER, Some("true")));
    assert!(!fits(DECISIONS_NEVER, Some("yes")));
    assert!(fits("plugin.notes", Some("on")));
    assert!(!fits("plugin.../x", Some("on")));
    assert!(!fits("plugin.notes", Some("maybe")));
    assert!(!fits("root_path", Some("/home/someone/thing")));
}

#[test]
fn what_a_project_shows_is_read_and_written_by_its_record() {
    let (dir, store) = store();
    let project = store
        .add_project(&dir.path().join("a"), Some(ORIGIN))
        .expect("add");
    let record = record_of(&store, &project);
    let projects = Projects { store: &store };

    projects.write(&record, ICON, Some("rocket")).expect("icon");
    projects
        .write(&record, COLOR, Some("#123abc"))
        .expect("colour");
    projects.write(&record, NAME, Some("Thing")).expect("name");
    projects.write(&record, GROUP, Some("Work")).expect("group");
    projects
        .write(&record, "plugin.notes", Some("off"))
        .expect("plugin");
    projects
        .write(&record, DECISIONS_NEVER, Some("true"))
        .expect("decisions");
    projects
        .write(&record, COLOR, Some("not a colour"))
        .expect("refused quietly");
    projects
        .write("origin:elsewhere", ICON, Some("star"))
        .expect("not here");

    let row = store.project(&project).expect("read").expect("there");
    assert_eq!(row.name, "Thing");
    assert_eq!(row.group.as_deref(), Some("Work"));
    assert_eq!(row.icon.as_deref(), Some("rocket"));
    assert_eq!(row.color.as_deref(), Some("#123abc"));
    assert_eq!(
        store.installed_plugins(&project).expect("plugins"),
        vec!["notes"]
    );
    assert!(store.enabled_plugins(&project).expect("plugins").is_empty());

    let here = projects.here().expect("here");
    let fields = &here[&record];
    let value = |field: &str| fields[field].value.clone();
    assert_eq!(value(NAME).as_deref(), Some("Thing"));
    assert_eq!(value("plugin.notes").as_deref(), Some("off"));
    assert_eq!(value(DECISIONS_NEVER).as_deref(), Some("true"));
    assert!(
        !fields.keys().any(|field| field.contains("path")),
        "a path never syncs: {:?}",
        fields.keys()
    );

    projects.write(&record, ICON, None).expect("cleared");
    projects
        .write(&record, DECISIONS_NEVER, Some("false"))
        .expect("decisions");
    let row = store.project(&project).expect("read").expect("there");
    assert_eq!(row.icon, None);
    assert_eq!(
        projects.here().expect("here")[&record][DECISIONS_NEVER]
            .value
            .as_deref(),
        Some("false")
    );
}
