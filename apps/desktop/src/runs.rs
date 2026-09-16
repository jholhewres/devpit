//! Starting a step, and recording what it did.
//!
//! A run is opened before the work begins and closed when it ends, so a card
//! never has work happening with nothing on screen to show for it. When the
//! process dies mid-run the row stays `running` and says so — which is honest,
//! and better than a row that silently reports success.
//!
//! **The work happens off the command's thread.** A turn takes seconds; doing
//! it inline would freeze the drag that started it for exactly as long, and a
//! board that locks up while it thinks is a board nobody drags things onto.
//! The command returns a `running` row and the thread finishes it, announcing
//! itself on `run:changed` so the card can catch up.
//!
//! Nothing here retries. A failed run leaves the card where it is with the
//! reason on it, and the next move is a person's.

use std::sync::Arc;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError, Run, RunState, Step, StepKind};
use tauri::AppHandle;

use crate::card_activity::{run_heard, run_reference, Doing};
use crate::in_flight::InFlight;
use crate::{steps, working};

/// Runs a step against a card, and returns the run it opened.
///
///
/// `came_from` is where a verdict sends the card back to — recorded on the
/// run's own row, so it survives the process that started it. Without it a review
/// that says "revise" leaves the card sitting in the reviewed column, which is
/// a review nobody acts on.
pub fn start(
    app: AppHandle,
    in_flight: Arc<InFlight>,
    store: &Store,
    card_id: &str,
    step: &Step,
    came_from: Option<&str>,
) -> Result<Run, RpcError> {
    start_chained(app, in_flight, store, card_id, step, came_from, 0)
}

/// The same, counting how many lanes this card has already passed through.
///
/// A person dropping a card starts at zero. The chain passes its own count
/// on, so a flow edited into a circle while a chain is in flight still stops.
pub fn start_chained(
    app: AppHandle,
    in_flight: Arc<InFlight>,
    store: &Store,
    card_id: &str,
    step: &Step,
    came_from: Option<&str>,
    hops: u8,
) -> Result<Run, RpcError> {
    // Nothing new once an update is about to go in. This is the single place a
    // run is born, so a card advancing on its own is refused here too — its
    // process would die with this one at the commit.
    if let Some(updating) = tauri::Manager::try_state::<crate::update::Updating>(&app) {
        if let Some(why) = crate::update::starting_refused(&updating.state()) {
            return Err(RpcError::new(ErrorCode::Conflict, why.to_owned()));
        }
    }

    let run_id = store.start_run(card_id, &step.id, came_from)?;
    // A session of its own for every run: two runs of one card are two
    // conversations, and one id for both would write the second over the first.
    let session_id = steps::fresh_session_id();
    if step.kind == StepKind::Agent {
        store.set_run_session(&run_id, &session_id)?;
    }
    // Heard in the one order everything else about the card is heard in.
    run_heard(
        &app,
        card_id,
        &run_reference(store, &run_id),
        Doing::Working,
    );

    let card = card_id.to_owned();
    let id = run_id.clone();
    // Read before the move: the row the command returns describes the step
    // that is about to run, and the thread takes ownership of the step itself.
    let step_id = step.id.clone();
    let step_name = step.name.clone();
    let step = step.clone();
    // The chain needs the same registry the run itself is watched in.
    let chained = Arc::clone(&in_flight);
    std::thread::spawn(move || {
        // Its own connection: SQLite handles are not shared across threads,
        // and the row it has to close is already committed.
        let Ok(store) = Store::open_default() else {
            return;
        };
        working::carry_out(
            working::Carrying {
                app,
                in_flight,
                chained,
                run_id: id,
                card_id: card,
                session_id,
                step,
                hops,
            },
            &store,
        );
    });

    Ok(Run {
        id: run_id,
        step_id,
        step_name,
        state: RunState::Running,
        output: None,
        exit_code: None,
        cost_usd: None,
        duration_ms: None,
        started_at: 0.0,
    })
}
