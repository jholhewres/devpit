// The window is the product on Windows and macOS; a console behind it leaks an
// implementation detail.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod board;
mod commands;
mod projects;
mod pty_bridge;
mod runs;
mod sessions;
mod settings;

use tauri_specta::{collect_commands, Builder};

/// Where the generated TypeScript lands.
///
/// Anchored to the manifest directory rather than the working directory:
/// `tauri dev` and a bare `./quockpit-desktop` run from different places, and a
/// relative path would quietly write the contract somewhere else.
const BINDINGS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/gen/bindings.ts");

/// The commands whose types are generated into TypeScript.
///
/// Separate from the invoke handler on purpose: one command cannot be in the
/// contract, because specta cannot describe the channel it streams over.
fn contract() -> Builder<tauri::Wry> {
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
        board::column_create,
        board::column_rename,
        board::column_reorder,
        board::column_delete,
        board::column_set_step,
        board::card_create,
        board::card_update,
        board::card_move,
        board::card_archive,
        board::step_create,
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

fn main() {
    // Regenerated on every dev run so `make dev` keeps the frontend types in
    // step while screens are being written. The test does the same thing and
    // fails when the committed file is stale, which is what covers a build
    // nobody ran the window for.
    #[cfg(debug_assertions)]
    contract()
        .export(specta_typescript::Typescript::default(), BINDINGS)
        .expect("export the contract");

    // The runtime handler carries everything, contract or not.
    //
    // `pty_drain` and `session_attach` stream over
    // `Channel<InvokeResponseBody>` so frames reach the webview as raw bytes
    // rather than JSON. specta cannot describe that enum; their wrappers are
    // hand-written. Everything else about a session — ensure, split, write,
    // resize — is in the generated contract.
    // The agents already installed on this machine become the starting set,
    // once, without ever overwriting one that has been edited.
    let seeded = runs::seed_agents();
    if seeded > 0 {
        println!("seeded {seeded} agents into ~/.quockpit/agents");
    }

    tauri::Builder::default()
        .manage(sessions::SessionState::new())
        .invoke_handler(tauri::generate_handler![
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
            board::column_create,
            board::column_rename,
            board::column_reorder,
            board::column_delete,
            board::column_set_step,
            board::card_create,
            board::card_update,
            board::card_move,
            board::card_archive,
            board::step_create,
            sessions::session_ensure,
            sessions::session_layout,
            sessions::session_focus,
            sessions::session_split,
            sessions::session_write,
            sessions::session_resize,
            sessions::session_detach,
            sessions::session_attach,
            settings::settings_read,
            settings::settings_write,
            settings::settings_finish_onboarding,
            pty_bridge::pty_drain,
        ])
        .run(tauri::generate_context!())
        .expect("the window did not open");
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
        assert_eq!(
            before, after,
            "web/src/gen/bindings.ts was stale — it has just been regenerated, commit it"
        );
    }
}
