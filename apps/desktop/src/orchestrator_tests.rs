use super::{free_folder, seed, slug, speaks_as};

#[test]
fn a_new_orchestrator_starts_with_its_brief_and_folders() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("orchestrator").join("claude2");
    seed(&folder).expect("seeded");
    let mine = std::fs::read_to_string(folder.join("CLAUDE.md")).expect("claude.md");
    assert!(
        mine.contains("@.devpit/orchestrator.md"),
        "CLAUDE.md does not import devpit's brief"
    );
    let brief = std::fs::read_to_string(folder.join(".devpit/orchestrator.md")).expect("brief");
    assert!(brief.contains("Never move a card into a lane that runs a step"));
    // The person answers a waiting session; the orchestrator never relays it.
    assert!(brief.contains("do\n  not relay \"go on\" as a message"));
    // Its folder is notes, organised, and never a repository.
    assert!(brief.contains("`context/projects/<project>.md`"));
    assert!(brief.contains("never initialise one or commit"));
    for kept in [
        "docs",
        "artifacts",
        "context",
        "context/projects",
        "decisions",
    ] {
        assert!(folder.join(kept).is_dir(), "{kept} is missing");
    }
}

#[test]
fn opening_again_leaves_what_the_person_changed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("claude2");
    seed(&folder).expect("seeded");
    std::fs::write(folder.join("CLAUDE.md"), "mine now").expect("edit");
    seed(&folder).expect("seeded again");
    assert_eq!(
        std::fs::read_to_string(folder.join("CLAUDE.md")).expect("brief"),
        "mine now"
    );
}

#[test]
fn devpits_half_of_the_brief_follows_the_build() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("claude2");
    seed(&folder).expect("seeded");
    std::fs::write(folder.join(".devpit/orchestrator.md"), "an older build's").expect("age it");
    seed(&folder).expect("opened again");
    let brief = std::fs::read_to_string(folder.join(".devpit/orchestrator.md")).expect("brief");
    assert!(
        brief.contains("devpit_sessions"),
        "the brief was left as an older build wrote it"
    );
}

#[test]
fn a_second_orchestrator_of_one_name_gets_a_folder_of_its_own() {
    let dir = tempfile::tempdir().expect("tempdir");
    let first = free_folder(dir.path(), "Client work").expect("a folder");
    assert!(first.ends_with("orchestrator/client-work"));
    std::fs::create_dir_all(&first).expect("made");
    let second = free_folder(dir.path(), "Client work").expect("a folder");
    assert!(second.ends_with("orchestrator/client-work-1"));
    assert_eq!(slug("   "), "orchestrator");
}

#[test]
fn an_orchestrator_says_its_account_where_the_folder_is_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let folder = dir.path().join("orchestrator").join("work");
    seed(&folder).expect("seeded");
    speaks_as(&folder, "01M39W5J").expect("said");
    assert_eq!(
        devpit_core::home::orchestrator_profile(&folder).as_deref(),
        Some("01M39W5J")
    );
    speaks_as(&folder, "claude").expect("changed");
    assert_eq!(
        devpit_core::home::orchestrator_profile(&folder).as_deref(),
        Some("claude")
    );
    // Notes, not a repository: nothing to push them to.
    assert!(!folder.join(".git").exists());
}
