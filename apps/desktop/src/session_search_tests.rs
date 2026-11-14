use super::*;

use devpit_core::Store;

/// The agent CLI's own folder for a project's transcripts.
fn cli_folder(installation: &Path, root: &Path) -> PathBuf {
    installation
        .join("projects")
        .join(devpit_agentcli::outside::folder_name(root))
}

fn transcript(dir: &Path, name: &str, lines: &[&str]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, lines.join("\n") + "\n").expect("transcript");
    path
}

#[test]
fn a_project_is_indexed_once_and_found_by_its_words() {
    let home = tempfile::tempdir().expect("tempdir");
    let installation = home.path().join(".claude");
    let root = Path::new("/work/demo.app");
    let dir = cli_folder(&installation, root);
    std::fs::create_dir_all(&dir).expect("mkdir");
    transcript(
        &dir,
        "aaa.jsonl",
        &[r#"{"type":"user","message":{"content":"rename the tokenizer module"}}"#],
    );

    let store = Store::open(&home.path().join("state.db")).expect("store");
    refresh(&store, "prj_1", root, std::slice::from_ref(&installation)).expect("refresh");
    let hits = search_index::search(store.conn(), "tokenizer", Some("prj_1"), 10).expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].session_id, "aaa");

    // Nothing moved: a second refresh reads nothing and changes nothing.
    refresh(&store, "prj_1", root, std::slice::from_ref(&installation)).expect("again");
    assert_eq!(
        search_index::search(store.conn(), "tokenizer", Some("prj_1"), 10)
            .expect("search")
            .len(),
        1
    );
}

#[test]
fn a_search_over_every_project_names_the_project_of_each_hit() {
    let home = tempfile::tempdir().expect("tempdir");
    let installation = home.path().join(".claude");
    let store = Store::open(&home.path().join("state.db")).expect("store");
    for (project, root, said) in [
        ("prj_1", "/work/one", "rename the tokenizer module"),
        ("prj_2", "/work/two", "the tokenizer drops a quote"),
    ] {
        let dir = cli_folder(&installation, Path::new(root));
        std::fs::create_dir_all(&dir).expect("mkdir");
        let line = format!(r#"{{"type":"user","message":{{"content":"{said}"}}}}"#);
        transcript(&dir, &format!("{project}.jsonl"), &[&line]);
        refresh(
            &store,
            project,
            Path::new(root),
            std::slice::from_ref(&installation),
        )
        .expect("refresh");
    }

    let mut found: Vec<(String, String)> =
        search_index::search(store.conn(), "tokenizer", None, 10)
            .expect("search")
            .into_iter()
            .map(|hit| (hit.project, hit.session_id))
            .collect();
    found.sort();
    assert_eq!(
        found,
        [
            ("prj_1".to_owned(), "prj_1".to_owned()),
            ("prj_2".to_owned(), "prj_2".to_owned())
        ]
    );
}
