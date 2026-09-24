use super::seed;

#[test]
fn a_new_orchestrator_starts_with_its_brief_and_folders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("orchestrator").join("claudin");
    seed(&folder).expect("seeded");
    let brief = std::fs::read_to_string(folder.join("CLAUDE.md")).expect("brief");
    assert!(brief.contains("Never move a card into a lane that runs a step"));
    for kept in ["docs", "artifacts", "context"] {
        assert!(folder.join(kept).is_dir(), "{kept} is missing");
    }
}

#[test]
fn opening_again_leaves_what_the_person_changed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("claudin");
    seed(&folder).expect("seeded");
    std::fs::write(folder.join("CLAUDE.md"), "mine now").expect("edit");
    seed(&folder).expect("seeded again");
    assert_eq!(
        std::fs::read_to_string(folder.join("CLAUDE.md")).expect("brief"),
        "mine now"
    );
}
