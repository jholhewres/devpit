//! The card's own checkout, and the terminal that opens on it.
//!
//! This is the join the board was missing. A card described a piece of work
//! and had no way to start doing it: the worktree was made only as a
//! side-effect of a step running, and the terminal knew nothing about cards.
//!
//! Both of those already existed; neither was reachable from a card. Nothing
//! here is new machinery — it is the two of them introduced to each other.

use std::path::PathBuf;

use std::sync::Arc;

use devpit_rpc::{CardTerminal, Checkout, ErrorCode, Played, RpcError};
use tauri::State;

use crate::projects::store;
use crate::sessions::{tab_for_card, SessionState};

/// What `card.detail` reports about a checkout, read fresh.
fn seen(path: PathBuf, base_ref: Option<String>) -> Checkout {
    if !path.is_dir() {
        return Checkout {
            path: path.display().to_string(),
            branch: None,
            base_ref,
            dirty_files: None,
            exists: false,
        };
    }
    let read = devpit_git::status(&path).ok();
    Checkout {
        branch: read.as_ref().map(|status| status.branch.clone()),
        dirty_files: read.as_ref().map(|status| status.dirty_files()),
        path: path.display().to_string(),
        base_ref,
        exists: true,
    }
}

/// `card.checkout` — makes the card's worktree, or reports the one it has.
///
/// Made rather than asked for: a card with no checkout has nowhere for an
/// agent to work, and the button that says "give this card a checkout" is the
/// whole of what somebody wants when they press it.
///
/// Blocking work off the UI thread: this can run `git worktree add` and then
/// a whole preparation — `pnpm install` is not something to hold a window for.
#[tauri::command]
#[specta::specta]
pub async fn card_checkout(card_id: String) -> Result<Checkout, RpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = store()?;
        let made = crate::checkout::checkout_of(&store, &card_id, |_| {})
            .map_err(|why| RpcError::new(ErrorCode::Internal, why))?;
        let base = store.card(&card_id)?.and_then(|row| row.base_ref);
        Ok::<_, RpcError>(seen(made, base))
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))?
}

/// `card.terminal` — a terminal open on this card's checkout.
///
/// Answers with the tab, which the window then shows. The agent is not started
/// here: the pane does not exist until the window has drawn it, and the tab
/// carries which agent to launch so it can be sent once the pane is there.
/// That seam already existed for the launcher palette; this reuses it rather
/// than inventing a second way in.
#[tauri::command]
#[specta::specta]
pub async fn card_terminal(
    state: State<'_, SessionState>,
    project_id: String,
    card_id: String,
) -> Result<CardTerminal, RpcError> {
    let wanted = card_id.clone();
    let cwd = tauri::async_runtime::spawn_blocking(move || {
        let store = store()?;
        crate::checkout::checkout_of(&store, &wanted, |_| {})
            .map_err(|why| RpcError::new(ErrorCode::Internal, why))
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))??;

    let tab_id = tab_for_card(&card_id);
    let layout = crate::sessions::ensure_at(&state, &project_id, &tab_id, &cwd)?;
    Ok(CardTerminal {
        layout,
        tab_id,
        card_id,
    })
}

/// What the bell says when a step with no undo starts, and nothing otherwise.
///
/// The one kind of run worth a line at the moment it *starts*: everything else
/// is told when it ends, and a deploy just set off is a thing to be able to see
/// was set off, by whom and on what, without waiting for it to come back.
///
/// It rings here rather than on the move, because a move no longer starts one:
/// `moving::what_runs` sends an irreversible step to this button, which asks
/// first.
pub(crate) fn bell_for(step_name: &str, irreversible: bool, card_title: &str) -> Option<String> {
    irreversible.then(|| format!("{step_name} started on \u{201c}{card_title}\u{201d}"))
}

/// `card.play` — runs this lane's step on this card, now.
///
/// One meaning, and only one. A lane with no step answers with that fact and
/// the screen offers a terminal instead — a button that does one thing when
/// the lane has a step and something else when it does not is a button with
/// two invisible meanings, and the lane's step name is right there to say
/// which would happen.
///
/// `confirmed` is the same word `card.move` uses, for the same reason: a step
/// with no undo is not fired by a click somebody might not have meant.
#[tauri::command]
#[specta::specta]
pub fn card_play(
    in_flight: State<Arc<crate::in_flight::InFlight>>,
    app: tauri::AppHandle,
    project_id: String,
    card_id: String,
    confirmed: bool,
) -> Result<Played, RpcError> {
    let store = store()?;
    let card = store
        .card(&card_id)?
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such card"))?;

    let Some(step) = store
        .columns(&project_id)?
        .into_iter()
        .find(|column| column.id == card.column_id)
        .and_then(|column| column.step_id)
        .and_then(|step_id| store.step(&step_id).ok().flatten())
    else {
        return Ok(Played {
            run: None,
            needs_confirming: false,
            lane_runs_nothing: true,
        });
    };

    // A run already going is work in flight. Starting a second one on the same
    // card is two processes writing one checkout.
    if store
        .runs(&card_id)?
        .iter()
        .any(|run| run.state == "running")
    {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            "something is already running on this card",
        ));
    }

    if step.irreversible && !confirmed {
        return Ok(Played {
            run: None,
            needs_confirming: true,
            lane_runs_nothing: false,
        });
    }

    if let Some(said) = bell_for(&step.name, step.irreversible, &card.title) {
        crate::notices::ring(
            &app,
            Some(&project_id),
            crate::notices::kind::IRREVERSIBLE,
            &said,
            Some("This step was marked as having no undo."),
            Some(&card_id),
        );
    }

    let shaped = devpit_rpc::Step {
        id: step.id.clone(),
        kind: crate::board::kind_of(&step.kind),
        name: step.name.clone(),
        config: step.config.clone(),
        irreversible: step.irreversible,
    };
    let run = crate::runs::start(
        app,
        Arc::clone(&in_flight),
        &store,
        &card_id,
        &shaped,
        // Played where it stands, so a refusal has nowhere else to send it.
        crate::run_from::Asking::first(Some(&card.column_id), devpit_core::store::Asked::Card),
    )?;

    Ok(Played {
        run: Some(run),
        needs_confirming: false,
        lane_runs_nothing: false,
    })
}

#[cfg(test)]
#[path = "card_work_tests.rs"]
mod tests;
