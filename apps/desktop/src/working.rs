//! Carrying out one step, on the thread that owns it.
//!
//! Apart from `runs.rs` because that file opens the row and hands it over, and
//! this is the work itself — which is most of the volume and none of the
//! bookkeeping.
//!
//! Everything here happens off the thread that draws. A step is a process, or
//! several; doing it inline would freeze the drag that started it for exactly
//! as long, and a board that locks up while it thinks is a board nobody drags
//! things onto.

use std::sync::Arc;

use devpit_core::Store;
use devpit_rpc::{Step, StepKind};
use tauri::{AppHandle, Emitter};

use crate::in_flight::InFlight;
use crate::{chaining, notices, steps};

/// Everything the thread needs, taken by value before it starts.
pub struct Carrying {
    pub app: AppHandle,
    pub in_flight: Arc<InFlight>,
    pub chained: Arc<InFlight>,
    pub run_id: String,
    pub card_id: String,
    pub step: Step,
    pub hops: u8,
}

/// Runs the step, records how it ended, and lets the board react.
pub fn carry_out(carrying: Carrying, store: &Store) {
    let Carrying {
        app,
        in_flight,
        chained,
        run_id: id,
        card_id: card,
        step,
        hops,
    } = carrying;

    let outcome = match step.kind {
        StepKind::Agent => {
            let progress = app.clone();
            let run = id.clone();
            let watching = Arc::clone(&in_flight);
            let watched = id.clone();
            steps::agent::run(
                store,
                &card,
                &step,
                |text| {
                    // The card shows work as it happens rather than a
                    // spinner that ends in a wall of text.
                    let _ = progress.emit("run:progress", (run.clone(), text.to_owned()));
                },
                |pid| watching.watch(&watched, pid),
            )
        }
        StepKind::Session => steps::session::start(store, &card, &step),
        StepKind::Command => steps::command::run(store, &card, &step),
    };

    let answered = match &outcome {
        Ok(finished) if finished.ok => Some(finished.output.clone()),
        _ => None,
    };
    let closed = match outcome {
        Ok(finished) => store.finish_run(
            &id,
            if finished.ok { "ok" } else { "failed" },
            Some(&finished.output),
            Some(finished.cost_usd),
            Some(finished.duration_ms),
            finished.exit_code.map(i64::from),
        ),
        Err(reason) => store.finish_run(&id, "failed", Some(&reason), None, None, None),
    };

    // A failure to record is worth saying out loud: the run finished and
    // the screen would otherwise show it running forever.
    if let Err(err) = closed {
        eprintln!("could not record the end of run {id}: {err}");
    }
    in_flight.forget(&id);

    // The bell. A run ending is the thing people leave the window for,
    // so it is the first thing the bell had to know. Only the ending —
    // starting one was a click they had just made.
    notices::run_ended(&app, store, &card, &step.name, answered.is_some());

    // What the board does about it. Backward on a refusal, forward on a
    // pass when the lane says so and is allowed to — see `advancing`.
    chaining::after(
        &app,
        &chained,
        store,
        &card,
        &step,
        answered.as_deref(),
        hops,
    );
    let _ = app.emit("run:changed", &card);
}
