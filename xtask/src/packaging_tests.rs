//! What the bundle promises, and the day it stops.

use super::*;

#[test]
fn an_updater_permission_on_the_window_is_named() {
    let granted = vec![
        "core:default".to_owned(),
        "dialog:allow-open".to_owned(),
        "updater:allow-check".to_owned(),
        "updater:default".to_owned(),
    ];
    assert_eq!(
        updater_permissions(&granted),
        ["updater:allow-check", "updater:default"]
    );
}

#[test]
fn a_window_that_asks_for_nothing_of_the_updater_passes() {
    let granted = vec!["core:default".to_owned(), "dialog:default".to_owned()];
    assert!(updater_permissions(&granted).is_empty());
}

/// And the tree as it stands: the identifier, the deb-only base, the overlay.
#[test]
fn the_tree_ships_what_it_says() {
    let found = the_bundle_says_what_it_ships(&crate::workspace_root());
    assert!(
        found.is_empty(),
        "{}",
        found
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );
}

/// The matrix, in the shape the workflow writes it.
const MATRIX: &str = "
    strategy:
      matrix:
        include:
          - os: ubuntu-22.04
            target: linux-x86_64
          - os: macos-14
            target: darwin-aarch64
  release:
    runs-on: ubuntu-24.04
";

#[test]
fn the_floor_and_its_arm_sibling_are_both_the_floor() {
    let matrix = MATRIX.replace(
        "- os: macos-14",
        "- os: ubuntu-22.04-arm\n            target: linux-aarch64\n          - os: macos-14",
    );
    assert!(above_the_floor(&matrix).is_empty());
}

#[test]
fn a_newer_image_on_the_build_leg_is_named() {
    let matrix = MATRIX.replace("- os: ubuntu-22.04", "- os: ubuntu-24.04");
    assert_eq!(above_the_floor(&matrix), ["ubuntu-24.04"]);
}

/// The one a guard cannot catch afterwards: it is not newer today.
#[test]
fn an_image_that_moves_on_its_own_is_named() {
    let matrix = MATRIX.replace("- os: ubuntu-22.04", "- os: ubuntu-latest");
    assert_eq!(above_the_floor(&matrix), ["ubuntu-latest"]);
}

/// The job that publishes builds nothing, so its image is not this guard's.
#[test]
fn the_publishing_job_is_not_a_builder() {
    assert!(above_the_floor(MATRIX).is_empty());
    assert!(MATRIX.contains("runs-on: ubuntu-24.04"));
}
