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
use devpit_rpc::{
    standing, CardFindings, CardReview, ErrorCode, Found, Review, RpcError, REVIEW_EVIDENCE,
};

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
pub async fn checkpoint_findings(run_id: String) -> Result<Found, RpcError> {
    crate::off_main::blocking(move || checkpoint_findings_now(run_id)).await
}

/// [`checkpoint_findings`], on the calling thread.
pub(crate) fn checkpoint_findings_now(run_id: String) -> Result<Found, RpcError> {
    findings(&crate::board::store()?, &run_id)
}

/// The reviews a card's runs left that found something, newest first, with
/// what the person set aside in each.
pub(crate) fn of_card(store: &Store, card_id: &str) -> Result<CardFindings, RpcError> {
    let mut reviews = Vec::new();
    for run in store.runs(card_id)? {
        let found = findings(store, &run.id)?;
        if found.findings.is_empty() {
            continue;
        }
        let dismissed = store.dismissed_findings(&run.id)?;
        reviews.push(CardReview {
            run_id: run.id,
            found,
            dismissed,
        });
    }
    Ok(CardFindings { reviews })
}

/// `card.findings` — what every review on this card found, for its diff.
#[tauri::command]
#[specta::specta]
pub async fn card_findings(card_id: String) -> Result<CardFindings, RpcError> {
    crate::off_main::blocking(move || of_card(&crate::board::store()?, &card_id)).await
}

/// `finding.dismiss` — sets one finding aside, or brings it back.
#[tauri::command]
#[specta::specta]
pub async fn finding_dismiss(run_id: String, at: u32, dismissed: bool) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::board::store()?;
        if store.run_state(&run_id)?.is_none() {
            return Err(RpcError::new(ErrorCode::NotFound, "no such run"));
        }
        Ok(store.dismiss_finding(&run_id, at, dismissed)?)
    })
    .await
}

#[cfg(test)]
#[path = "checkpoint_findings_tests.rs"]
mod tests;
