//! The command list, and the TypeScript generated from it.
//!
//! Separate from the window: which commands exist is not a question about how
//! the window is drawn, and keeping the list here means adding one does not
//! touch the file that opens the app.

use tauri_specta::{collect_commands, Builder};

use crate::{board, columns, commands, files, front, projects, sessions, settings, steps};

/// Where the generated TypeScript lands.
///
/// Anchored to the manifest directory rather than the working directory:
/// `tauri dev` and a bare `./quockpit-desktop` run from different places, and a
/// relative path would quietly write the contract somewhere else.
pub const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/gen/bindings.ts");

/// The commands whose types are generated into TypeScript.
///
/// Separate from the invoke handler on purpose: one command cannot be in the
/// contract, because specta cannot describe the channel it streams over.
pub fn contract() -> Builder<tauri::Wry> {
    Builder::<tauri::Wry>::new().commands(collect_commands![
        commands::app_info,
        commands::app_health,
        commands::app_capabilities,
        projects::project_list,
        projects::project_add,
        projects::project_clone,
        projects::project_open,
        projects::project_tree,
        projects::project_changes,
        projects::project_history,
        projects::project_notes,
        projects::project_note_add,
        board::board_get,
        columns::column_create,
        columns::column_rename,
        columns::column_reorder,
        columns::column_delete,
        columns::column_set_step,
        board::card_create,
        board::card_update,
        board::card_move,
        files::file_read,
        files::file_write,
        front::card_archive,
        front::card_diff,
        columns::step_create,
        steps::agents_list,
        sessions::terminal_attach_agent,
        sessions::session_ensure,
        sessions::session_layout,
        sessions::session_focus,
        sessions::session_split,
        sessions::session_write,
        sessions::session_resize,
        sessions::session_detach,
        settings::settings_read,
        settings::settings_write,
        settings::settings_finish_onboarding,
    ])
}

#[cfg(test)]
mod tests {
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
}
