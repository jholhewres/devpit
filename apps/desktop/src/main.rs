// The window is the product on Windows and macOS; a console behind it leaks an
// implementation detail.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod account;
mod adopted_history;
mod adopting;
mod advancing;
mod agent_api;
mod agent_reach;
mod arranging;
mod asking;
mod attaching;
mod attaching_chat;
mod blocks;
mod board;
mod branches;
mod browser;
mod browser_cookies;
mod browser_driving;
mod browser_menu;
// Where tauri draws with GTK, which is every unix that is not macOS. A child
// webview is placed by hand there; the module says why.
#[cfg(all(unix, not(target_os = "macos")))]
mod browser_gtk;
mod claims;
mod cloning;
mod columns;
mod commands;
#[cfg(all(unix, not(target_os = "macos")))]
mod frame_gtk;
mod live_sessions;
mod orchestrator;
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
mod chat_relay;
mod chat_running;
mod chat_turn;
mod checkout;
mod checkpoint;
mod checkpoint_findings;
mod checkpoint_preview;
mod claude_plugin;
#[cfg(any(debug_assertions, test))]
mod contract;
mod cycles;
mod diffs;
mod error_reports;
mod error_sender;
mod files;
mod filetree;
mod front;
mod handler;
mod happening;
mod heads_down;
mod history;
mod in_flight;
mod index;
#[cfg(target_os = "linux")]
mod input_method;
mod installations;
mod kept_out;
mod kinds;
mod listener;
mod live;
mod mcp;
mod mcp_reading;
mod moving;
mod notices;
mod off_main;
mod openers;
mod outside_sessions;
mod pane_screen;
mod panels;
mod panes;
mod pasting;
mod paths;
mod plan_limits;
mod plugin_data;
mod plugins;
mod post;
mod prime;
mod project_naming;
mod projects;
mod question;
mod reading_path;
mod receipts;
mod reconcile;
mod refusing;
mod regrouping;
mod restoring;
mod reveal;
mod roots;
mod run_from;
mod runs;
mod saves;
mod search;
mod session_search;
mod sessions;
mod settings;
mod shell_launch;
mod shell_probe;
mod skills;
mod slash;
mod sources;
mod spend_history;
mod staging;
mod steering;
mod steps;
mod still_holds;
mod stopping_a_run;
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
    // `devpit agent …` and `devpit mcp` are an agent reaching the running
    // app, not a second window: answered here, before anything starts GTK.
    if let Some(code) = agent_door() {
        std::process::exit(code);
    }

    // Before anything starts GTK, and before any thread exists.
    #[cfg(target_os = "linux")]
    input_method::use_the_desktops();

    // Kept only once the switch is on; installed first so none is missed.
    devpit_core::reports::keep_panics();

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
        // The terminal's copy and paste. The webview's own clipboard is not
        // reachable from a terminal on WebKitGTK, and it cannot read a picture.
        .plugin(tauri_plugin_clipboard_manager::init())
        // Checking and downloading are the app's; installing is not — see
        // `update` for why only an AppImage is ever installed from here.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(chat::Talking::default())
        .manage(chat_relay::Relay::default())
        .manage(steering::Steering::default())
        .manage(asking::Asking::default())
        .manage(blocks::Blocks::default())
        .manage(update::Updating::default())
        /* Empty, and filled only by somebody granting a pane. */
        .manage(browser_driving::Granted::default())
        .manage(browser::Sessions::default())
        /* What the browser menu was last opened for, so its window can ask. */
        .manage(browser_menu::Opening::default())
        .manage(error_sender::Presence::new(devpit_core::reports::now()))
        // Whether somebody is at the window, for reports that only go when
        // nobody is.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Focused(focused) = event {
                if window.label() == "main" {
                    if let Some(presence) =
                        tauri::Manager::try_state::<error_sender::Presence>(window)
                    {
                        presence.focus(*focused, devpit_core::reports::now());
                    }
                }
            }
        })
        .setup(|app| {
            // Managed here and not in the builder because it holds the handle
            // it relays through, and the handle does not exist until now.
            tauri::Manager::manage(app, sessions::SessionState::new(app.handle().clone()));
            // The window rebuilt around a container that can hold a page
            // *beside* the app rather than under it. Done now, while the
            // window has exactly one webview in it, so the swap has nothing to
            // disturb; see `browser_gtk` for what GTK does otherwise.
            #[cfg(all(unix, not(target_os = "macos")))]
            if let Some(main) = tauri::Manager::get_webview_window(app, "main") {
                browser_gtk::settle(&main);
                frame_gtk::repaint_on_state_change(&main);
            }
            // A development build says so where the window is listed —
            // the taskbar, alt-tab — because the window draws its own frame
            // and the title is otherwise seen nowhere. The top bar's badge is
            // the other half, for when both devpits are on screen.
            #[cfg(debug_assertions)]
            if let Some(main) = tauri::Manager::get_webview_window(app, "main") {
                let _ = main.set_title("devpit (dev)");
            }
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
            // The home made private before anything opens or writes in it. A
            // first start creates it owner-only; an install from before this
            // existed is world-readable until someone tightens it, and this
            // start is the only moment that knows about every file in there.
            // What it could not do is said, not fatal.
            if let Ok(root) = devpit_core::Store::root() {
                if let Err(err) = devpit_core::home::make_private_root(&root) {
                    eprintln!("{} could not be created private: {err}", root.display());
                }
                for (path, err) in devpit_core::home::harden(&root) {
                    eprintln!("{} was left as it was: {err}", path.display());
                }
                error_reports::resume(&root);
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
            // What the cards were doing before this process started: asked
            // once, on its own thread, for the project the window opens on.
            card_reconcile::rebuild_on_start(app.handle().clone());
            // Looks for a newer devpit while the switch says to. Started
            // last: it is the one thing here that can wait.
            update::watch(app.handle().clone());
            // Sends the error reports the person switched on, when idle.
            error_sender::watch(app.handle().clone());
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

/// The exit code of an agent's command, when the arguments are one.
fn agent_door() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let first = args.first()?.as_str();
    if first != "agent" && first != "mcp" {
        return None;
    }
    let root = match devpit_core::Store::root() {
        Ok(root) => root,
        Err(err) => {
            eprintln!("devpit: cannot find where devpit keeps its state: {err}");
            return Some(1);
        }
    };
    Some(match first {
        "mcp" => devpit_agentapi::mcp::serve(&root),
        _ => devpit_agentapi::cli::run(&root, &args[1..]),
    })
}
