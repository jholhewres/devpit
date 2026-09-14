use super::*;

use devpit_core::Store;

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
    let dir = installation
        .join("projects")
        .join(devpit_agentcli::outside::folder_name(root));
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
