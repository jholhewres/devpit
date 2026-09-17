//! A review an agent gave, kept as the run's evidence.
//!
//! Apart from `agent.rs` because writing a review down is not running a turn,
//! and because the turn already has as much as one file should carry.
//!
//! The rule is `devpit_rpc::reviewed`, which is pure and tested on its own.
//! This is the part that touches the world: it asks the repository which
//! revision the review was made against, and writes the payload.

use devpit_core::Store;

/// Writes the review into the run's evidence, when the answer is one.
///
/// Anchored to the revision it was made against, so a later look says the
/// finding is outdated rather than re-pointing its line at whatever is on that
/// line now. Silent when it cannot: evidence that fails to save does not fail
/// a run, and a run with none reads as having left none.
pub(crate) fn kept_as_a_review(
    store: &Store,
    run_id: &str,
    answer: &str,
    rubric: &str,
    cwd: &std::path::Path,
) {
    let Some(review) = devpit_rpc::reviewed(
        answer,
        devpit_git::head_of(cwd).ok(),
        (!rubric.is_empty()).then(|| rubric.to_owned()),
    ) else {
        return;
    };
    let Ok(payload) = serde_json::to_string(&review) else {
        return;
    };
    if let Err(err) = store.record_evidence(
        run_id,
        &devpit_core::store::Evidence {
            version: devpit_rpc::REVIEW_EVIDENCE,
            payload,
        },
    ) {
        eprintln!("could not keep run {run_id}'s review: {err}");
    }
}
