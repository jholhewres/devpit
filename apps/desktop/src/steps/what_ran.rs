//! The circumstances a run happened in, recorded while they are still true.
//!
//! Gathered here rather than inside the step because it has to be gathered
//! **before** the command starts. A command that writes a commit moves HEAD,
//! and reading the revision afterwards would record the code the run produced
//! instead of the code it ran against — a green row pointing at the wrong
//! commit, which is worse than a row pointing at none.
//!
//! Nothing here fails a run. A repository that will not answer `rev-parse` is
//! a revision nobody knows, and `None` is what the screen says.

use std::path::Path;

use devpit_core::store::Ran;
use devpit_core::Store;
use devpit_steps::Context;

/// What is true about this run right now.
///
/// The base revision comes off the context, which took it from the card: it is
/// the commit the card's front began at, and no repository remembers that once
/// the base branch moves. Empty there means a card that never had a checkout,
/// which is `None` here and `unknown` on screen.
pub fn gathered(command: &str, cwd: &Path, context: &Context, in_a_worktree: bool) -> Ran {
    Ran {
        command: Some(command.to_owned()),
        in_directory: Some(cwd.display().to_string()),
        // The names, never the values: whatever a profile put in a step's
        // environment is not something a run gets to keep a copy of.
        declared_env: context
            .environment()
            .into_iter()
            .map(|(name, _)| name)
            .collect(),
        base_revision: said(&context.base_ref),
        head_revision: devpit_git::head_of(cwd).ok(),
        // Read before the command too: this is half of what says, later,
        // whether the result is still about the code in front of somebody.
        saw_changes: devpit_git::standing_at(cwd).ok(),
        in_a_worktree: Some(in_a_worktree),
    }
}

/// An empty string in the context is a value nobody set, not a value that is
/// empty — the context has no way to say the difference, and this does.
fn said(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

/// Records it against the run, and says so when it cannot.
///
/// A snapshot that fails to save must not fail the run: the work is the point,
/// and a row whose circumstances are unknown is exactly what [`Ran::unknown`]
/// is for.
pub fn recorded(store: &Store, run_id: &str, ran: &Ran) {
    if let Err(err) = store.record_what_ran(run_id, ran) {
        eprintln!("could not record what run {run_id} ran: {err}");
    }
}

#[cfg(test)]
#[path = "what_ran_tests.rs"]
mod tests;
