// The window is the product on Windows and macOS; a console behind it leaks an
// implementation detail.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod account;
mod adopting;
mod advancing;
mod arranging;
mod asking;
mod attaching;
mod board;
mod branches;
mod claims;
mod columns;
mod commands;
mod rewinding;
mod runs_list;
// Only ever compiled where it is used. The contract exists to generate the
// frontend's types — in a release build nothing calls it, and a module dead in
// release should say so rather than warn about it on every build.
mod agent_choice;
mod agent_profiles;
mod card_activity;
mod card_chat;
mod card_reconcile;
mod card_route;
mod card_sessions;
mod card_work;
mod cards;
mod chaining;
mod chat;
mod chat_running;
mod chat_turn;
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
mod installations;
mod kinds;
mod listener;
mod live;
mod mcp;
mod mcp_reading;
mod moving;
mod notices;
mod openers;
mod outside_sessions;
mod panels;
mod panes;
mod pasting;
mod paths;
mod plan_limits;
mod plugin_data;
mod plugins;
mod post;
mod prime;
mod priming;
mod projects;
mod question;
mod receipts;
mod reconcile;
mod refusing;
mod restoring;
mod reveal;
mod roots;
mod runs;
mod saves;
mod search;
mod session_search;
mod sessions;
mod settings;
mod shell_launch;
mod skills;
mod slash;
mod sources;
mod spend_history;
mod staging;
mod steering;
mod steps;
mod tap;
mod threads;
mod turn_changes;
mod update;
mod update_deb;
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
    // `session_attach` streams over `Channel<InvokeResponseBody>` so frames
    // reach the webview as raw bytes rather than JSON. specta cannot describe
    // that enum, so its wrapper is hand-written. Everything else about a
    // session — ensure, split, write, resize — is in the generated contract.
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        // Checking and downloading are the app's; installing is not — see
        // `update` for why only an AppImage is ever installed from here.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(chat::Talking::default())
        .manage(steering::Steering::default())
        .manage(asking::Asking::default())
        .manage(update::Updating::default())
        .setup(|app| {
            // Managed here and not in the builder because it holds the handle
            // it relays through, and the handle does not exist until now.
            tauri::Manager::manage(app, sessions::SessionState::new(app.handle().clone()));
            // The catalogue is compiled in and pinned by a test, so a manifest
            // breaking the contract is a build defect: loud where it is being
            // written; logged in release, where `plugin_data` refuses it on use
            // and panicking would only take the whole app away.
            if let Err(err) = devpit_rpc::validate_catalogue(&devpit_rpc::catalogue()) {
                if cfg!(debug_assertions) {
                    panic!("the plugin catalogue breaks the contract: {err}");
                }
                eprintln!("the plugin catalogue breaks the contract: {err}");
            }
            // The home made private before anything opens or writes in it. An
            // install from before this existed is world-readable until someone
            // tightens it, and this start is the only moment that knows about
            // every file in there. What it could not do is said, not fatal.
            if let Ok(root) = devpit_core::Store::root() {
                for (path, err) in devpit_core::home::harden(&root) {
                    eprintln!("{} was left as it was: {err}", path.display());
                }
            }
            // Project folders named and moved before anything reads one. Here
            // and not in `Store::open`, so a test opening a store moves
            // nothing; a failure leaves each project where it was.
            if let Ok(root) = devpit_core::Store::root() {
                if let Err(err) = devpit_core::Store::open_default()
                    .and_then(|store| devpit_core::home::settle(&store, &root))
                {
                    eprintln!("project folders were not settled: {err}");
                }
            }
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
            // Looks for a newer devpit while the switch says to. Started
            // last: it is the one thing here that can wait.
            update::watch(app.handle().clone());
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
