//! One version, and every way a second one gets written down.

use super::*;

#[test]
fn the_workspace_version_is_the_one_read() {
    let manifest = "[workspace]\nmembers = []\n\n[workspace.package]\nversion = \"0.1.0\"\nedition = \"2021\"\n";
    assert_eq!(workspace_version(manifest).as_deref(), Some("0.1.0"));
    assert_eq!(workspace_version("[workspace]\n").as_deref(), None);
}

/// The sabotage the story names: `web/package.json` back at 0.0.0.
#[test]
fn a_package_file_that_says_something_else_is_refused() {
    assert!(disagreeing("0.1.0", Some("0.0.0"))
        .expect("a disagreement")
        .contains("says 0.0.0"));
    assert_eq!(disagreeing("0.1.0", Some("0.1.0")), None);
    assert!(disagreeing("0.1.0", None)
        .expect("a missing version")
        .contains("no version"));
}

#[test]
fn a_manifest_that_writes_the_version_again_is_refused() {
    let manifest = "[package]\nname = \"devpit-desktop\"\nversion.workspace = true\n\n\
                    [dependencies]\ndevpit-core = { version = \"0.1.0\", path = \"../../crates/core\" }\n";
    assert_eq!(repeated_lines(manifest, "0.1.0"), vec![6]);

    let inherited = manifest.replace("version = \"0.1.0\", ", "");
    assert!(repeated_lines(&inherited, "0.1.0").is_empty());
}

/// And the real tree passes, which is what makes the three above worth having.
#[test]
fn the_tree_says_it_once() {
    let root = crate::workspace_root();
    let found = the_version_has_one_source(&root);
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
