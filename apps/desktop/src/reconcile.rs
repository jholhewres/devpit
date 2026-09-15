//! What the last process left behind.
//!
//! Apart from `runs.rs` because it is the opposite job: that file starts work
//! and watches it, and this one cleans up after work that was never watched to
//! its end. It runs once, at launch, before anything draws.

use devpit_core::Store;
use tauri::AppHandle;

use crate::card_activity::{run_heard, run_reference, state_of_run};
use crate::notices;

/// Closes the runs whose process is gone, and says so in the bell.
///
/// At launch, once. Nothing survives the process that spawned its thread, so
/// a `running` row after a restart is not work still happening — and a card
/// that says it is working forever is worse than one that says it failed.
pub fn close_abandoned(app: &AppHandle) {
    let Ok(store) = Store::open_default() else {
        return;
    };
    let Ok(stranded) = store.close_abandoned_runs() else {
        return;
    };
    for (run_id, card_id) in &stranded {
        run_heard(
            app,
            card_id,
            &run_reference(&store, run_id),
            state_of_run("lost"),
        );
        let title = store
            .card(card_id)
            .ok()
            .flatten()
            .map(|row| row.title)
            .unwrap_or_else(|| "a card".to_owned());
        notices::ring(
            app,
            store.project_id_of_card(card_id).ok().flatten().as_deref(),
            notices::kind::RUN,
            &format!("A run on “{title}” was lost"),
            Some("The app closed while it was running, so how it ended is unknown."),
            Some(card_id),
        );
    }
}
