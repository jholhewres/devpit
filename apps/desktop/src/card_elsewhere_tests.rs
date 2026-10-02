use std::path::Path;

use super::{note, Kept};

#[test]
fn a_file_outside_the_cards_checkout_is_kept_by_its_repository() {
    let dir = tempfile::tempdir().expect("dir");
    let home = dir.path().join("card");
    let other = dir.path().join("devpit-app");
    for repo in [&home, &other] {
        std::fs::create_dir_all(repo.join(".git")).expect("repo");
    }
    std::fs::create_dir_all(other.join("crates/api")).expect("folder");
    let mut kept = Kept::default();

    // Inside its own checkout: Changes already shows it.
    assert!(!note(&mut kept, &home, &home.join("src/lib.rs")));
    // Elsewhere, once each.
    assert!(note(
        &mut kept,
        &home,
        &other.join("crates/api/client_ip.rs")
    ));
    assert!(!note(
        &mut kept,
        &home,
        &other.join("crates/api/client_ip.rs")
    ));
    // Relative or outside any repository: nothing to name it by.
    assert!(!note(&mut kept, &home, Path::new("src/x.rs")));
    assert!(!note(&mut kept, &home, Path::new("/nowhere/file.rs")));

    let other_root = std::fs::canonicalize(&other)
        .expect("canon")
        .display()
        .to_string();
    assert_eq!(
        kept.repos.get(&other_root).map(Vec::as_slice),
        Some(&["crates/api/client_ip.rs".to_owned()][..])
    );
}
