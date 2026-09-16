//! The terminals outlive the window, and this is what says so.

use super::*;

#[test]
fn the_app_as_it_stands_leaves_the_server_alone() {
    let found = the_app_never_kills_the_tmux_server(&crate::workspace_root());
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

/// And it is not a guard that passes because it looks at nothing: a file that
/// says it is caught.
#[test]
fn a_file_that_kills_the_server_is_caught() {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = dir.path().join("apps/desktop/src");
    std::fs::create_dir_all(&app).expect("tree");
    std::fs::write(
        app.join("quitting.rs"),
        "fn leave() {\n    let _ = server.kill_server();\n}\n",
    )
    .expect("a file");

    let found = the_app_never_kills_the_tmux_server(dir.path());

    let said = found
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ");
    assert_eq!(found.len(), 1, "{said}");
    assert_eq!(found[0].line, 2, "{said}");
}
