use super::refusals;

/// The pair that shipped five failed releases: same name, different case,
/// different extension. The extension is no protection — the resolver is
/// given a name and tries the extensions itself.
#[test]
fn two_files_whose_names_differ_only_in_case_are_refused() {
    let said = refusals(["web/src/shell/acts.ts", "web/src/shell/Acts.tsx"].into_iter());
    assert_eq!(said.len(), 1);
    assert!(
        said[0].what.contains("macOS and Windows"),
        "{}",
        said[0].what
    );
}

/// Same name in different folders is two names, and ordinary.
#[test]
fn the_same_name_in_another_folder_is_fine() {
    assert!(refusals(["web/src/shell/tree.ts", "web/src/files/tree.ts"].into_iter()).is_empty());
}

/// A test file names itself: `./acts` never reaches `acts.test.ts`.
#[test]
fn a_test_beside_its_module_is_not_a_collision() {
    assert!(
        refusals(["web/src/shell/acts.ts", "web/src/shell/acts.test.ts"].into_iter()).is_empty()
    );
}

/// Rust too, where `mod` guesses the same way.
#[test]
fn it_looks_at_rust_and_leaves_prose_alone() {
    assert_eq!(
        refusals(["crates/core/src/home.rs", "crates/core/src/Home.rs"].into_iter()).len(),
        1
    );
    assert!(refusals(["README.md", "readme.md"].into_iter()).is_empty());
}
