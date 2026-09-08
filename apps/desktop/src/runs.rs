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

use quockpit_agentcli as agent;
use std::sync::Arc;

use quockpit_core::Store;
use quockpit_rpc::{RpcError, Run, RunState, Step, StepKind};
use tauri::{AppHandle, Emitter};

use crate::in_flight::InFlight;
use crate::steps;

/// Runs a step against a card, and returns the run it opened.
///
///
/// `came_from` is where a verdict sends the card back to. Without it a review
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
    let run_id = store.start_run(card_id, &step.id)?;

    let card = card_id.to_owned();
    let id = run_id.clone();
    let back_to = came_from.map(ToOwned::to_owned);
    // Read before the move: the row the command returns describes the step
    // that is about to run, and the thread takes ownership of the step itself.
    let step_id = step.id.clone();
    let step_name = step.name.clone();
    let step = step.clone();
    std::thread::spawn(move || {
        // The thread opens its own connection: SQLite handles are not shared
        // across threads, and the row it has to close is already committed.
        let Ok(store) = Store::open_default() else {
            return;
        };

        let outcome = match step.kind {
            StepKind::Agent => {
                let progress = app.clone();
                let run = id.clone();
                let watching = Arc::clone(&in_flight);
                let watched = id.clone();
                steps::agent::run(
                    &store,
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
            StepKind::Session => steps::session::start(&store, &card, &step),
            StepKind::Command => steps::command::run(&store, &card, &step),
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

        // The verdict is the only thing in the product that moves a card on
        // its own, and it only ever moves it backwards.
        if let (Some(column), Some(answer)) = (&back_to, &answered) {
            if let Some(why) = steps::sends_back(&step.config, answer) {
                let position = store.cards_in_column(column).unwrap_or(0);
                let _ = store.move_card(&card, column, position);
                let _ = store.note_on_card(&card, &format!("sent back — {why}"));
            }
        }
        let _ = app.emit("run:changed", &card);
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

/// Copies the agents already installed on this machine into `~/.quockpit`.
///
/// Runs on every start and never overwrites, so an agent the person edited
/// stays theirs.
pub fn seed_agents() -> usize {
    let Ok(dir) = crate::steps::agents_dir() else {
        return 0;
    };
    agent::seed_sources()
        .iter()
        .filter_map(|source| agent::seed_agents(&dir, source).ok())
        .sum()
}
