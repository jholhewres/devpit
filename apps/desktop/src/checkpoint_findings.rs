//! `checkpoint.findings` — what a review found, and whether it still points there.
//!
//! Apart from `checkpoint.read` because it carries the payload. A list of runs
//! asks what each one proved; only the one somebody opened asks for the
//! findings, and sending them with every row would put megabytes through the
//! bridge for a panel nobody opened.
//!
//! The anchoring rule is [`devpit_rpc::standing`], which is pure and tested on
//! its own: a review made against another revision reads as outdated, and its
//! line numbers are not re-pointed at whatever is on those lines now.

use devpit_core::Store;
use devpit_rpc::{standing, ErrorCode, Found, Review, RpcError, REVIEW_EVIDENCE};

pub(crate) fn findings(store: &Store, run_id: &str) -> Result<Found, RpcError> {
    if store.run_state(run_id)?.is_none() {
        return Err(RpcError::new(ErrorCode::NotFound, "no such run"));
    }
    let Some(evidence) = store.evidence_of(run_id)? else {
        return Ok(Found::none());
    };
    // A shape this build does not know is not one to read as though it were
    // the one it does.
    if evidence.version != REVIEW_EVIDENCE {
        return Ok(Found::none());
    }
    let Ok(review) = serde_json::from_str::<Review>(&evidence.payload) else {
        return Ok(Found::none());
    };

    // Where the code stands now, asked of the directory the run worked in. A
    // run with none recorded has nowhere to ask, which `standing` reads as
    // unanchored rather than as agreement.
    let now = store
        .what_ran(run_id)?
        .and_then(|ran| ran.in_directory)
        .and_then(|cwd| devpit_git::head_of(std::path::Path::new(&cwd)).ok());

    Ok(Found {
        standing: standing(&review, now.as_deref()),
        at_revision: review.at_revision.clone(),
        now,
        rubric: review.rubric.clone(),
        findings: review.findings,
    })
}

#[tauri::command]
#[specta::specta]
pub fn checkpoint_findings(run_id: String) -> Result<Found, RpcError> {
    findings(&crate::board::store()?, &run_id)
}

#[cfg(test)]
#[path = "checkpoint_findings_tests.rs"]
mod tests;
