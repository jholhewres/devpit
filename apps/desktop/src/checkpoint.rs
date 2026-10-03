//! `checkpoint.read` — what one run proves, and whether it still proves it.
//!
//! Asked per run rather than per page. Answering it touches the repository
//! twice — the revision and where the tree stands — and doing that for fifty
//! rows of a list would put two hundred processes between somebody and their
//! own history.
//!
//! Nothing here decides anything: the rules are [`devpit_rpc::verdict`] and
//! [`devpit_rpc::validity`], which are pure and tested on their own. This
//! reads the row and asks them.

use devpit_core::store::Evidence;
use devpit_core::store::{Asked, Carried, Ran, WhoseRun};
use devpit_core::Store;
use devpit_rpc::{
    verdict, Checked, ErrorCode, Report, Review, RpcError, Tested, WhatRan, Whose, REVIEW_EVIDENCE,
    TESTS_EVIDENCE,
};

use crate::still_holds::still_holds;

/// What a run reported, read back out of its evidence.
///
/// Reads a review (blocking findings fail) or a test report. Neither means
/// no report, which [`devpit_rpc::verdict`] never calls a pass.
fn reported(evidence: Option<&Evidence>) -> Option<Report> {
    let evidence = evidence?;
    if evidence.version == TESTS_EVIDENCE {
        let tested: Tested = serde_json::from_str(&evidence.payload).ok()?;
        return Some(Report {
            passed: tested.passed,
            failed: tested.failed,
        });
    }
    if evidence.version != REVIEW_EVIDENCE {
        // A shape this build does not know is not a shape to read as though it
        // were the one it does.
        return None;
    }
    let review: Review = serde_json::from_str(&evidence.payload).ok()?;
    let failed = devpit_rpc::blocking(&review) as u32;
    Some(Report {
        // A review that found nothing looked at something: it is one check,
        // and it passed. A review with blocking findings is one check that
        // failed, whatever else it also noted.
        passed: u32::from(failed == 0),
        failed: u32::from(failed > 0),
    })
}

pub(crate) fn checked(store: &Store, run_id: &str) -> Result<Checked, RpcError> {
    let Some(state) = store.run_state(run_id)? else {
        return Err(RpcError::new(ErrorCode::NotFound, "no such run"));
    };
    let ran = store.what_ran(run_id)?.unwrap_or_else(Ran::unknown);
    let evidence = store.evidence_of(run_id)?;

    Ok(Checked {
        run_id: run_id.to_owned(),
        state: crate::board::state_of(&state),
        verdict: verdict(crate::board::state_of(&state), reported(evidence.as_ref())),
        validity: still_holds(&ran),
        ran: ran.is_known().then(|| as_read(&ran)),
        evidence_version: evidence.map(|left| left.version as f64),
        whose: as_whose(&store.whose_run(run_id)?.unwrap_or_else(WhoseRun::unknown)),
    })
}

/// The snapshot as the screen reads it. A separate type from the store's
/// because the contract forbids 64-bit integers and owns its own names.
fn as_read(ran: &Ran) -> WhatRan {
    WhatRan {
        command: ran.command.clone(),
        in_directory: ran.in_directory.clone(),
        declared_env: ran.declared_env.clone(),
        base_revision: ran.base_revision.clone(),
        head_revision: ran.head_revision.clone(),
        in_a_worktree: ran.in_a_worktree,
    }
}

/// Whose a run was, as the screen reads it. Absent rather than a word for
/// "nobody said": a blank is read as unknown, and a word would be read as an
/// answer.
fn as_whose(whose: &WhoseRun) -> Whose {
    Whose {
        asked: match whose.asked {
            Asked::Board => Some("board"),
            Asked::Card => Some("card"),
            Asked::Checkpoint => Some("checkpoint"),
            Asked::Chain => Some("chain"),
            Asked::Unknown => None,
        }
        .map(str::to_owned),
        asked_from: whose.asked_from.clone(),
        carried: match &whose.carried {
            Carried::Process => Some("process".to_owned()),
            Carried::Agent { .. } => Some("agent".to_owned()),
            Carried::Unknown => None,
        },
        profile: match &whose.carried {
            Carried::Agent { profile } => profile.clone(),
            _ => None,
        },
    }
}

#[tauri::command]
#[specta::specta]
pub async fn checkpoint_read(run_id: String) -> Result<Checked, RpcError> {
    crate::off_main::blocking(move || checkpoint_read_now(run_id)).await
}

/// [`checkpoint_read`], on the calling thread.
pub(crate) fn checkpoint_read_now(run_id: String) -> Result<Checked, RpcError> {
    checked(&crate::board::store()?, &run_id)
}

#[cfg(test)]
#[path = "checkpoint_tests.rs"]
mod tests;
