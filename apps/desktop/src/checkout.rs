//! Which directory a step runs in, and how a card gets one of its own.
//!
//! Not every step wants a checkout: refining and reviewing read the card's
//! text, and handing them a worktree costs minutes and gigabytes for nothing.

use std::path::PathBuf;

use devpit_core::Store;
use devpit_rpc::{Step, StepKind};
use serde::Deserialize;

use crate::prime;

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Wants {
    needs_worktree: Option<bool>,
}

/// Whether a step wants a checkout of its own.
///
/// Declared by the step, defaulted by its kind — and never decided by the
/// column's name. A column gets renamed, and a rule that reads the name breaks
/// silently when it does.
pub fn needs_worktree(kind: StepKind, config: &str) -> bool {
    let declared: Wants = serde_json::from_str(config).unwrap_or_default();
    declared.needs_worktree.unwrap_or(match kind {
        // A session is a person sitting in a shell; a command builds or tests.
        StepKind::Session | StepKind::Command => true,
        // An agent step reads and writes the card, most of the time.
        StepKind::Agent => false,
    })
}

/// The directory this step should run in.
///
/// Creates the card's worktree the first time one is needed, reuses it after,
/// and prepares it once. Steps that need no checkout get the project itself.
pub fn cwd_for(
    store: &Store,
    card_id: &str,
    step: &Step,
    on_line: impl FnMut(&str),
) -> Result<PathBuf, String> {
    let project = store
        .project_of_card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("this card has no project on disk")?;

    if !needs_worktree(step.kind, &step.config) {
        return Ok(PathBuf::from(&project));
    }
    checkout_of(store, card_id, on_line)
}

/// The card's own checkout, made the first time it is asked for.
///
/// Split out of `cwd_for` because a step is no longer the only thing that
/// wants one: opening a terminal on a card asks the same question, and a
/// person clicking that button is asking for the same folder a step would get.
pub fn checkout_of(
    store: &Store,
    card_id: &str,
    mut on_line: impl FnMut(&str),
) -> Result<PathBuf, String> {
    let project = store
        .project_of_card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("this card has no project on disk")?;
    let main = PathBuf::from(&project);

    let card = store
        .card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("no such card")?;

    // Already has one: reuse it. A second worktree per card would be a second
    // answer to "where is this card's work".
    if let Some(existing) = card.worktree_path.as_deref() {
        let path = PathBuf::from(existing);
        if path.is_dir() {
            return Ok(path);
        }
    }

    let project_id = store
        .project_id_of_card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("this card has no project")?;
    let home = Store::root().map_err(|err| err.to_string())?;
    let base = store
        .preference(devpit_core::preference::WORKTREE_BASE)
        .map_err(|err| err.to_string())?
        .unwrap_or_default();
    let at = devpit_git::worktree_at(&base, &home, &main, &project_id, card_id);
    let branch = devpit_git::branch_for(&card.title, card_id);

    let made = devpit_git::create(&main, &at, &branch, "HEAD").map_err(|err| err.to_string())?;
    store
        .set_card_front(card_id, made.path.to_str(), Some(made.base_ref.as_str()))
        .map_err(|err| err.to_string())?;

    let declared = prime::read(
        &devpit_core::home::ProjectHome::of(store, &home, &project_id)
            .map_err(|err| err.to_string())?
            .prime(),
    );
    on_line("preparing the worktree");
    match prime::run(&declared, &main, &made.path, &mut on_line).map_err(|err| err.to_string())? {
        prime::Primed::Failed { command, code } => {
            // Named as the preparation's failure. Read as the work's, it sends
            // someone looking at the agent for a problem in `pnpm install`.
            Err(format!(
                "preparing the worktree failed: `{command}` exited {code}"
            ))
        }
        _ => Ok(made.path),
    }
}

#[cfg(test)]
#[path = "checkout_tests.rs"]
mod tests;
