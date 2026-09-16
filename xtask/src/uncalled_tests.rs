//! A command in the contract that no screen calls.

use super::*;

fn said(found: &[Finding]) -> String {
    found
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// A tree with one registration list and one screen.
fn tree(registered: &[&str], window: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let app = dir.path().join("apps/desktop/src");
    std::fs::create_dir_all(&app).expect("tree");
    let body = registered
        .iter()
        .map(|name| format!("        {name},\n"))
        .collect::<String>();
    std::fs::write(
        app.join("contract_list.rs"),
        format!("pub fn loaded() {{\n    collect![\n{body}    ]\n}}\n"),
    )
    .expect("a list");
    let web = dir.path().join("web/src/shell");
    std::fs::create_dir_all(&web).expect("tree");
    std::fs::write(web.join("Screen.tsx"), window).expect("a screen");
    dir
}

#[test]
fn the_contract_as_it_stands_is_all_called_or_listed() {
    let found = a_command_has_a_caller(&crate::workspace_root());
    assert!(found.is_empty(), "{}", said(&found));
}

#[test]
fn a_command_nothing_calls_is_caught_by_name_and_line() {
    let dir = tree(
        &["cards::card_board", "board::board_get"],
        "void commands.boardGet(id)\n",
    );

    // Nothing is allowed in this tree: the reasons in the real list are about
    // the real contract, and a fixture that read them would be testing it.
    let found = uncalled_in(dir.path(), "");

    assert_eq!(found.len(), 1, "{}", said(&found));
    assert_eq!(found[0].line, 3, "{}", said(&found));
    assert!(found[0].what.contains("card_board"), "{}", said(&found));
}

/// The generated client spells it in camel, and that is what a screen types.
#[test]
fn a_command_the_window_calls_is_left_alone() {
    let dir = tree(&["cards::card_board"], "void commands.cardBoard(id)\n");
    assert!(
        uncalled_in(dir.path(), "").is_empty(),
        "a called command was reported"
    );
}

/// A hand-written invoke is still a caller. Rare, and not a reason to fail.
#[test]
fn a_hand_written_invoke_counts_as_a_caller() {
    let dir = tree(&["cards::card_board"], "invoke(\"card_board\", { id })\n");
    assert!(
        uncalled_in(dir.path(), "").is_empty(),
        "an invoked command was reported"
    );
}

#[test]
fn a_command_with_a_reason_is_left_alone() {
    let dir = tree(&["chat::chat_frames"], "nothing here\n");
    let found = uncalled_in(dir.path(), "chat_frames carries Ask and Frame\n");
    assert!(found.is_empty(), "{}", said(&found));
}

/// The other direction: a reason for a command that is called again, or gone.
#[test]
fn a_reason_nobody_needs_is_reported() {
    let dir = tree(&["cards::card_board"], "void commands.cardBoard(id)\n");

    let stale = uncalled_in(dir.path(), "card_board it is called, actually\n");
    assert!(
        stale[0].what.contains("has a caller now"),
        "{}",
        said(&stale)
    );

    let gone = uncalled_in(dir.path(), "chat_frames long gone\n");
    assert!(
        gone.iter().any(|one| one.what.contains("not a command")),
        "{}",
        said(&gone)
    );
}

/// The list is read the way the other lists are: comments and blanks are not
/// entries.
#[test]
fn comments_and_blank_lines_are_not_commands() {
    let dir = tree(&["cards::card_board"], "void commands.cardBoard(id)\n");
    let found = uncalled_in(dir.path(), "# a comment\n\n   \n");
    assert!(found.is_empty(), "{}", said(&found));
}
