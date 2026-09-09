// The window is the product on Windows and macOS; a console behind it leaks an
// implementation detail.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod asking;
mod board;
mod branches;
mod claims;
mod columns;
mod commands;
// Only ever compiled where it is used. The contract exists to generate the
// frontend's types — in a release build nothing calls it, and a module dead in
// release should say so rather than warn about it on every build.
mod chat;
mod checkout;
#[cfg(any(debug_assertions, test))]
mod contract;
mod diffs;
mod files;
mod front;
mod handler;
mod happening;
mod in_flight;
mod index;
mod kinds;
mod listener;
mod mcp;
mod panes;
mod post;
mod prime;
mod projects;
mod pty_bridge;
mod question;
mod reveal;
mod roots;
mod runs;
mod saves;
mod sessions;
mod settings;
mod staging;
mod steps;
mod threads;
mod workspace;
mod worktrees;

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
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(chat::Talking::default())
        .manage(asking::Asking::default())
        .setup(|app| {
            // Managed here and not in the builder because it holds the handle
            // it relays through, and the handle does not exist until now.
            tauri::Manager::manage(app, sessions::SessionState::new(app.handle().clone()));
            // Hooks are how the board hears about work as it happens rather
            // than a poll later. Started here so the endpoint is on disk before
            // the first turn goes out.
            if let Ok(root) = devpit_core::Store::root() {
                listener::start(app.handle().clone(), &root);
            }
            Ok(())
        })
        .manage(std::sync::Arc::new(in_flight::InFlight::new()))
        .invoke_handler(handler::handler())
        .run(tauri::generate_context!())
        .expect("the window did not open");
}
