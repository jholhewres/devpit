use super::*;

#[test]
fn a_skill_this_machine_does_not_have_is_named() {
    let installed = vec!["tdd".to_owned(), "research".to_owned()];
    let wanted = vec!["tdd".to_owned(), "tddd".to_owned()];
    assert_eq!(missing(&wanted, &installed), vec!["tddd".to_owned()]);
}

#[test]
fn every_skill_installed_is_accepted() {
    let installed = vec!["tdd".to_owned()];
    assert!(missing(&["tdd".to_owned()], &installed).is_empty());
}

#[test]
fn a_directory_without_a_skill_file_is_not_a_skill() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bare = dir.path().join("not-a-skill");
    std::fs::create_dir_all(&bare).expect("create");
    assert!(!is_skill(&bare));

    let real = dir.path().join("tdd");
    std::fs::create_dir_all(&real).expect("create");
    std::fs::write(real.join("SKILL.md"), "# tdd\n").expect("write");
    assert!(is_skill(&real));
}
