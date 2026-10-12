//! A project's team layers from the window: fetch each one's ref and show
//! what approving it changes, then the person's click pins it. Both are the
//! person's: an agent never fetches a layer, and nothing fetched applies
//! until the lockfile pins it.

use std::path::Path;

use devpit_devflow::folder::{self, State};
use devpit_devflow::pinning::{self, Change};
use devpit_rpc::{DevflowScreen, DevflowTeamChange, DevflowTeamChanges, ErrorCode, RpcError};
use tauri::AppHandle;

fn refused(why: String) -> RpcError {
    RpcError::new(ErrorCode::Invalid, why)
}

fn change_of(change: Change) -> DevflowTeamChange {
    DevflowTeamChange {
        name: change.name,
        git: change.git,
        from_ref: change.from_ref,
        from_commit: change.from_commit,
        to_ref: change.to_ref,
        to_commit: change.to_commit,
        files: change
            .files
            .into_iter()
            .map(|(path, how)| format!("{how} {path}"))
            .collect(),
        diff: change.diff,
    }
}

/// The layers the project's own file extends.
fn extends(root: &Path) -> Result<Vec<devpit_devflow::config::Extend>, RpcError> {
    match folder::read(root) {
        State::Valid(_, own) => Ok(own.extends),
        _ => Err(refused(
            "devflow.yaml does not read: fix it before its layers".to_owned(),
        )),
    }
}

/// `devflow.team_fetch` — each layer at the ref devflow.yaml names, and
/// what approving it would change. Nothing is applied.
#[tauri::command]
#[specta::specta]
pub async fn devflow_team_fetch(project_id: String) -> Result<DevflowTeamChanges, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        let home = crate::devflow::home();
        let changes = extends(&root)?
            .iter()
            .map(|layer| pinning::fetched(&root, layer, &home).map(change_of))
            .collect::<Result<Vec<_>, _>>()
            .map_err(refused)?;
        Ok(DevflowTeamChanges { changes })
    })
    .await
}

/// `devflow.team_approve` — the person's click: the layer pinned at the
/// commit they were shown, in the project's devflow.lock. devpit never
/// commits it.
#[tauri::command]
#[specta::specta]
pub async fn devflow_team_approve(
    app: AppHandle,
    project_id: String,
    git: String,
    commit: String,
) -> Result<DevflowScreen, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let project = crate::devflow_screen::project_here(&store, &project_id)?;
        let root = Path::new(&project.root_path);
        let layer = extends(root)?
            .into_iter()
            .find(|layer| layer.git == git)
            .ok_or_else(|| refused(format!("devflow.yaml does not extend {git}")))?;
        pinning::approve(root, &layer, &commit, &crate::devflow::home()).map_err(refused)?;
        crate::devflow_asks::rang(Some(&app), &project.id);
        Ok(crate::devflow_screen::screen(Some(&app), &store, &project))
    })
    .await
}
