//! What a finished run does about the card, once the rule has decided.
//!
//! The rule is `advancing::decide` and it takes plain values; this reads the
//! board to fill them in and writes the result. Apart from `runs.rs` because
//! that file is about running a step and this is about the board reacting to
//! one having run.
//!
//! # There is no loop here, and that is the design
//!
//! Automatic mode is not a daemon and not a timer. When a run ends, the thread
//! that ran it already knows the outcome and already moved the card backwards
//! on a refusal — moving it forwards on a pass is the same code path, one
//! branch further along. If the lane it lands in has a step, that move starts
//! it, and the chain continues by the mechanism that already existed.
//!
//! What that costs, said rather than hidden: a run in flight dies when the app
//! quits. `reconcile` closes those at the next launch.

use std::sync::Arc;

use devpit_core::Store;
use devpit_rpc::Step;
use tauri::AppHandle;

use crate::cycles::MOST_HOPS;
use crate::in_flight::InFlight;

use crate::advancing::{decide, verdict_of, Autonomy, Lane, Move};
use crate::notices;

/// Reads the lane a card is standing in, as the rule needs it.
fn lane_of(
    store: &Store,
    project_id: &str,
    column_id: &str,
) -> Option<(Autonomy, Option<String>, bool)> {
    let columns = store.columns(project_id).ok()?;
    let here = columns.iter().find(|column| column.id == column_id)?;

    // Whether the destination runs something with no undo. Read here, because
    // the rule takes an answer and not a database.
    let target_irreversible = here
        .on_pass
        .as_ref()
        .and_then(|to| columns.iter().find(|column| &column.id == to))
        .and_then(|target| target.step_id.clone())
        .and_then(|step_id| store.step(&step_id).ok().flatten())
        .map(|step| step.irreversible)
        .unwrap_or(false);

    Some((
        Autonomy::parse(&here.autonomy),
        here.on_pass.clone(),
        target_irreversible,
    ))
}

/// Applies whatever the rule decided.
///
/// Errors are swallowed deliberately: this runs after the work is already
/// done and recorded, and a card that failed to move is a card in the wrong
/// column — not a run that should be reported as having failed.
pub fn after(
    app: &AppHandle,
    in_flight: &Arc<InFlight>,
    store: &Store,
    card_id: &str,
    step: &Step,
    answered: Option<&str>,
    hops: u8,
) {
    let Ok(Some(card)) = store.card(card_id) else {
        return;
    };
    let Ok(Some(project_id)) = store.project_id_of_card(card_id) else {
        return;
    };
    let Some((autonomy, on_pass, target_is_irreversible)) =
        lane_of(store, &project_id, &card.column_id)
    else {
        return;
    };

    // Where it came from is read off the run's own row now, not carried in
    // this process's memory — see migration 007.
    let came_from = store
        .runs(card_id)
        .ok()
        .and_then(|runs| runs.first().map(|run| run.id.clone()))
        .and_then(|run_id| store.run_came_from(&run_id).ok().flatten());

    let lane = Lane {
        autonomy,
        on_pass: on_pass.as_deref(),
        target_is_irreversible,
    };
    let verdict = verdict_of(step, answered);

    match decide(&lane, &verdict, came_from.as_deref()) {
        Move::Stay => {}
        Move::Back { to, why } => {
            let at = store.cards_in_column(&to).unwrap_or(0);
            let _ = store.move_card(card_id, &to, at);
            let _ = store.note_on_card(card_id, &format!("sent back — {why}"));
            notices::ring(
                app,
                Some(&project_id),
                notices::kind::RUN,
                &format!("“{}” was sent back", card.title),
                Some(&why),
                Some(card_id),
            );
        }
        Move::Forward { to } => {
            let at = store.cards_in_column(&to).unwrap_or(0);
            let _ = store.move_card(card_id, &to, at);
            let _ = store.note_on_card(card_id, &format!("{} passed", step.name));
            notices::ring(
                app,
                Some(&project_id),
                notices::kind::RUN,
                &format!("“{}” moved on", card.title),
                Some(&format!("{} passed", step.name)),
                Some(card_id),
            );
            // And the lane it landed in runs whatever it runs. This is the
            // chain: no timer, no daemon, no loop — the thread that finished
            // one step starts the next, exactly as a person dropping the card
            // there would have.
            onward(app, in_flight, store, card_id, &to, &project_id, hops);
        }
        /* Told, not taken. The work passed and the person should hear that
        they can take the next step — the bell is where that goes, because
        it is the thing they check when they were not watching. */
        Move::Offer { to } => {
            let where_to = store
                .columns(&project_id)
                .ok()
                .and_then(|columns| {
                    columns
                        .into_iter()
                        .find(|column| column.id == to)
                        .map(|column| column.name)
                })
                .unwrap_or_else(|| "the next lane".to_owned());
            notices::ring(
                app,
                Some(&project_id),
                notices::kind::RUN,
                &format!("“{}” is ready for {where_to}", card.title),
                Some(&format!(
                    "{} passed. Move it when you are ready.",
                    step.name
                )),
                Some(card_id),
            );
        }
    }
}

/// Starts the destination lane's step, if it has one and may still run.
///
/// The hop cap is a backstop, not a design. `column_set_flow` refuses to save
/// a flow that closes a loop, so a chain should never come round — but the
/// board can be edited while one is in flight, and a card moving for ever
/// with nobody watching is the worst thing this mode can do.
fn onward(
    app: &AppHandle,
    in_flight: &Arc<InFlight>,
    store: &Store,
    card_id: &str,
    column_id: &str,
    project_id: &str,
    hops: u8,
) {
    if hops >= MOST_HOPS {
        let _ = store.note_on_card(
            card_id,
            "the chain stopped here — it had moved through too many lanes",
        );
        notices::ring(
            app,
            Some(project_id),
            notices::kind::RUN,
            "A card stopped moving on its own",
            Some("It passed through too many lanes in a row. Check the board's flow."),
            Some(card_id),
        );
        return;
    }

    let Ok(columns) = store.columns(project_id) else {
        return;
    };
    let Some(next) = columns
        .into_iter()
        .find(|column| column.id == column_id)
        .and_then(|column| column.step_id)
        .and_then(|step_id| store.step(&step_id).ok().flatten())
    else {
        return;
    };

    // `decide` already refuses to enter an irreversible lane automatically, so
    // reaching one here would mean the board changed underneath the chain.
    // Checked again rather than assumed: this starts work that spends money.
    if next.irreversible {
        return;
    }

    let step = devpit_rpc::Step {
        id: next.id.clone(),
        kind: crate::board::kind_of(&next.kind),
        name: next.name.clone(),
        config: next.config.clone(),
        irreversible: next.irreversible,
    };
    let _ = crate::runs::start_chained(
        app.clone(),
        Arc::clone(in_flight),
        store,
        card_id,
        &step,
        Some(column_id),
        hops + 1,
    );
}
