//! The contract is the frontend's types, and it has to stay in step.

use super::*;

/// Writes the TypeScript contract, and fails when it was out of date.
///
/// Generating from `main` meant the frontend types were only refreshed by
/// someone opening the window — a command could reach `main` and never
/// reach the screen, which is the drift this rule exists to stop. As a
/// test it runs in `make test` and in CI, so a contract change that was
/// not regenerated fails the build rather than the next screen.
#[test]
fn the_typescript_contract_is_up_to_date() {
    let before = std::fs::read_to_string(BINDINGS).unwrap_or_default();

    contract()
        .export(specta_typescript::Typescript::default(), BINDINGS)
        .expect("export the contract");

    let after = std::fs::read_to_string(BINDINGS).expect("read back");

    // Compared as a boolean, not with assert_eq: the two sides are the
    // whole file, and printing them turns one stale line into a thousand
    // lines of noise nobody reads.
    assert!(
        before == after,
        "web/src/gen/bindings.ts was stale — it has just been regenerated, commit it \
         ({} lines before, {} after)",
        before.lines().count(),
        after.lines().count()
    );
}
