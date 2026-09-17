//! What the release job promises, and each way of breaking it.

use super::*;

fn ours() -> String {
    std::fs::read_to_string(crate::workspace_root().join(WORKFLOW)).expect("the workflow")
}

#[test]
fn the_workflow_as_it_stands_keeps_them() {
    let broken = refusals(&ours());
    assert!(
        broken.is_empty(),
        "{}",
        broken
            .iter()
            .map(|(line, what)| format!("{line}: {what}"))
            .collect::<Vec<_>>()
            .join("; ")
    );
}

/// An action pinned to a tag is an action somebody else can change under a job
/// that holds the signing key.
#[test]
fn an_action_pinned_to_a_tag_is_refused() {
    let loosened = ours().replace(
        "actions/checkout@fbc6f3992d24b796d5a048ff273f7fcc4a7b6c09",
        "actions/checkout@v5",
    );
    assert!(refusals(&loosened)
        .iter()
        .any(|(_, what)| what.contains("not pinned")));
}

#[test]
fn the_key_in_a_second_step_is_refused() {
    let spread = ours().replace(
        "      - name: Publish the artifacts\n",
        "      - name: Publish the artifacts\n        # TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}\n",
    );
    assert!(refusals(&spread)
        .iter()
        .any(|(_, what)| what.contains("hands the signing key")));
}

#[test]
fn publishing_without_verify_tag_is_refused() {
    let loose = ours().replace(" --verify-tag", "");
    assert!(refusals(&loose)
        .iter()
        .any(|(_, what)| what.contains("--verify-tag")));
}

/// The manifests are what an installed app polls; published first, they name
/// files that are not there yet.
#[test]
fn manifests_published_before_the_artifacts_are_refused() {
    let text = ours();
    let created = text.find("gh release create").expect("a create step");
    let swapped = format!(
        "{}gh release upload before\n{}",
        &text[..created],
        &text[created..]
    );
    assert!(refusals(&swapped)
        .iter()
        .any(|(_, what)| what.contains("before the files they name")));
}

/// The review's case: with the key in the environment of a step that runs
/// `pnpm build`, a compromised devDependency reads it and signs an update
/// every installed app accepts.
#[test]
fn the_key_next_to_the_frontend_build_is_refused() {
    let together = ours().replace("--config '{\"build\":{\"beforeBuildCommand\":\"\"}}'", "");
    assert!(
        refusals(&together)
            .iter()
            .any(|(_, what)| what.contains("also builds the frontend")),
        "the keyed step building the frontend was accepted"
    );
}

#[test]
fn a_checkout_that_keeps_the_token_is_refused() {
    let kept = ours().replace("persist-credentials: false", "persist-credentials: true");
    assert!(refusals(&kept)
        .iter()
        .any(|(_, what)| what.contains(".git/config")));
}

/// A release that does not wait for the end-to-end suite ships whatever the
/// unit tests missed.
#[test]
fn a_release_that_publishes_without_testing_is_refused() {
    let untested = ours().replace("      - run: make test\n", "");
    let broken = refusals(&untested);
    assert!(
        broken
            .iter()
            .any(|(_, what)| what.contains("without running the tests")),
        "a release that never runs the tests was accepted: {broken:?}"
    );
}

/// The called workflow runs inside the release, so its actions are pinned too.
#[test]
fn the_called_end_to_end_workflow_is_pinned() {
    let called = std::fs::read_to_string(crate::workspace_root().join(CALLED)).expect("e2e.yml");
    assert!(unpinned(&called).is_empty(), "{:?}", unpinned(&called));
    let loosened = called.replace(
        "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
        "actions/upload-artifact@v4",
    );
    assert!(!unpinned(&loosened).is_empty());
}
