//! The rule discard has to keep: a path outside the project is refused
//! before git ever sees it.

use super::*;

#[test]
fn changes_discard_refuses_a_path_that_climbs_out_of_the_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("inside.txt"), "x").expect("write");

    let refused = resolved(dir.path(), &["../../../etc/passwd".to_owned()]);
    assert!(
        matches!(refused, Err(err) if err.code == ErrorCode::Forbidden),
        "an escaping path was allowed through"
    );
}

#[test]
fn changes_discard_allows_a_path_inside_the_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("inside.txt"), "x").expect("write");

    assert!(resolved(dir.path(), &["inside.txt".to_owned()]).is_ok());
}

/// The bug this guards: `resolve` canonicalises, which fails for a file that
/// is not there — and a deleted file is the single most common thing anyone
/// discards. `resolved` must accept it so the restore can run at all.
#[test]
fn changes_discard_allows_a_path_whose_file_was_already_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let gone = dir.path().join("gone.txt");
    std::fs::write(&gone, "x").expect("write");
    std::fs::remove_file(&gone).expect("remove");

    assert!(resolved(dir.path(), &["gone.txt".to_owned()]).is_ok());
}
