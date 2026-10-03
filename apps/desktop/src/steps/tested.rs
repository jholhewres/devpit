//! A command step's report folder, and its reports kept as evidence.

use std::path::{Path, PathBuf};

use devpit_core::home::ProjectHome;
use devpit_core::store::Evidence;
use devpit_core::Store;

/// An empty folder for this run's `$DEVPIT_REPORT_DIR`. Emptied first, so an
/// earlier run's report is never read as this one's.
pub(crate) fn fresh_folder(
    store: &Store,
    root: &Path,
    card_id: &str,
    run_id: &str,
) -> Option<PathBuf> {
    if run_id.is_empty()
        || !run_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let project_id = store.project_id_of_card(card_id).ok()??;
    let folder = ProjectHome::of(store, root, &project_id)
        .ok()?
        .reports()
        .join(run_id);
    let _ = std::fs::remove_dir_all(&folder);
    std::fs::create_dir_all(&folder).ok()?;
    Some(folder)
}

/// Keeps what the run reported as its evidence and removes the folder.
pub(crate) fn kept(
    store: &Store,
    run_id: &str,
    folder: Option<&Path>,
    printed: Option<&str>,
    cwd: &Path,
) {
    let tested = devpit_steps::report::read(folder, printed, cwd);
    if let Some(folder) = folder {
        let _ = std::fs::remove_dir_all(folder);
    }
    let Some(mut tested) = tested else {
        return;
    };
    // Failure text is output too: known secrets come out.
    let secrets = crate::kept_out::what_devpit_gave(store);
    for failure in &mut tested.failures {
        failure.name = crate::kept_out::kept_out(&failure.name, &secrets);
        failure.message = failure
            .message
            .as_deref()
            .map(|said| crate::kept_out::kept_out(said, &secrets));
    }
    let Ok(payload) = serde_json::to_string(&tested) else {
        return;
    };
    let evidence = Evidence {
        version: devpit_rpc::TESTS_EVIDENCE,
        payload,
    };
    if let Err(err) = store.record_evidence(run_id, &evidence) {
        eprintln!("could not keep run {run_id}'s test report: {err}");
    }
}
