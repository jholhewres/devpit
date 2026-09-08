// The window is the product on Windows and macOS; a console behind it leaks an
// implementation detail.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod projects;
mod pty_bridge;
mod sessions;
mod settings;

use tauri_specta::{collect_commands, Builder};

fn main() {
    // Two roles, deliberately separated.
    //
    // This builder owns the *contract*: the commands whose types are generated
    // into TypeScript. It does not own the invoke handler, because one command
    // cannot be in it.
    let contract = Builder::<tauri::Wry>::new().commands(collect_commands![
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
    ]);

    // TypeScript types are emitted here, from the Rust side, on every dev
    // build. That is what stops the two sides from diverging: there is no
    // second place where the type is written.
    //
    // quockpit to the manifest directory rather than the working directory:
    // `tauri dev` and a bare `./quockpit-desktop` run from different places,
    // and a relative path would quietly write the contract somewhere else.
    #[cfg(debug_assertions)]
    contract
        .export(
            specta_typescript::Typescript::default(),
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/src/gen/bindings.ts"),
        )
        .expect("export contract types");

    // The runtime handler carries everything, contract or not.
    //
    // `pty_drain` and `session_attach` stream over
    // `Channel<InvokeResponseBody>` so frames reach the webview as raw bytes
    // rather than JSON. specta cannot describe that enum; their wrappers are
    // hand-written. Everything else about a session — ensure, split, write,
    // resize — is in the generated contract.
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
