// The window is the product on Windows and macOS; a console behind it leaks an
// implementation detail.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod board;
mod columns;
mod commands;
// Only ever compiled where it is used. The contract exists to generate the
// frontend's types — in a release build nothing calls it, and a module dead in
// release should say so rather than warn about it on every build.
#[cfg(any(debug_assertions, test))]
mod contract;
mod files;
mod front;
mod projects;
mod pty_bridge;
mod runs;
mod sessions;
mod settings;
mod steps;

fn main() {
    // Regenerated on every dev run so `make dev` keeps the frontend types in
    // step while screens are being written. The test does the same thing and
    // fails when the committed file is stale, which is what covers a build
    // nobody ran the window for.
    #[cfg(debug_assertions)]
    contract::contract()
        .export(specta_typescript::Typescript::default(), contract::BINDINGS)
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
        .plugin(tauri_plugin_dialog::init())
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
            sessions::session_attach,
            settings::settings_read,
            settings::settings_write,
            settings::settings_finish_onboarding,
            pty_bridge::pty_drain,
        ])
        .run(tauri::generate_context!())
        .expect("the window did not open");
}
