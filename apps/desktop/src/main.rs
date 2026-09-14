// The window is the product on Windows and macOS; a console behind it leaks an
// implementation detail.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod account;
mod advancing;
mod arranging;
mod asking;
mod board;
mod branches;
mod claims;
mod columns;
mod commands;
// Only ever compiled where it is used. The contract exists to generate the
// frontend's types — in a release build nothing calls it, and a module dead in
// release should say so rather than warn about it on every build.
mod agent_choice;
mod agent_profiles;
mod card_work;
mod cards;
mod chaining;
mod chat;
mod checkout;
#[cfg(any(debug_assertions, test))]
mod contract;
mod cycles;
mod diffs;
mod files;
mod filetree;
mod front;
mod handler;
mod happening;
mod history;
mod in_flight;
mod index;
mod kinds;
mod listener;
mod live;
mod mcp;
mod moving;
mod notices;
mod openers;
mod panels;
mod panes;
mod paths;
mod post;
mod prime;
mod priming;
mod projects;
mod pty_bridge;
mod question;
mod reconcile;
mod refusing;
mod reveal;
mod roots;
mod runs;
mod saves;
mod search;
mod sessions;
mod settings;
mod shell_launch;
mod sources;
mod staging;
mod steps;
mod tap;
mod threads;
mod watching;
mod working;
mod workspace;
mod worktree_base;
mod worktrees;
mod wsfiles;

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
            // Which agent CLIs this machine has, asked of the login shell —
            // which costs an interactive shell startup. Started now so the
            // first time somebody opens the menu the answer is already there.
            shell_launch::warm_installed();
            // A run still marked `running` after a restart is a run whose
            // thread died with the last process. Closed here, before anything
            // draws, or the card says it is working forever.
            reconcile::close_abandoned(app.handle());
            Ok(())
        })
        .manage(std::sync::Arc::new(in_flight::InFlight::new()))
        .invoke_handler(handler::handler())
        .run(tauri::generate_context!())
        .expect("the window did not open");
}
