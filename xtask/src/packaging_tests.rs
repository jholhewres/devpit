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
