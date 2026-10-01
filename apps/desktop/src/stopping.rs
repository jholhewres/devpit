//! Stopping a session of an orchestrator's account: the agent ends, and so
//! does the terminal it ran in — its pane closed the way the window closes
//! one, its tab too when it was the last pane — so nothing is left running
//! out of sight.
//!
//! Never on the orchestrator's own initiative: the tool says so, and the
//! brief. Work in flight is lost, and that is the person's call.

use devpit_rpc::{ErrorCode, RpcError};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

/// Said when a tab was closed from here, so a window showing it lets it go.
pub(crate) const TAB_CLOSED: &str = "session:tab-closed";

/// `orchestrator.stop` — the window's own way to stop a session.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_stop(
    app: AppHandle,
    profile_id: String,
    name: String,
) -> Result<String, RpcError> {
    crate::off_main::blocking(move || {
        stop(&app, &profile_id, &name, None)
            .map(|said| said["stopped"].as_str().unwrap_or_default().to_owned())
    })
    .await
}

/// `terminal.close` — a devpit terminal closed from outside its tab: the
/// pane, or its tab when it is the last one, and whatever runs in it.
#[tauri::command]
#[specta::specta]
pub async fn terminal_close(
    app: AppHandle,
    project_id: String,
    pane_id: String,
) -> Result<(), RpcError> {
    crate::off_main::blocking(move || close_pane(&app, &project_id, &pane_id).map(|_| ())).await
}

/// Closes a pane the way the window does — its tab too when it was the last
/// one — and answers the tab it was in.
pub(crate) fn close_pane(
    app: &AppHandle,
    project_id: &str,
    pane_id: &str,
) -> Result<String, RpcError> {
    let store = crate::projects::store()?;
    let (tab_id, panes) = store
        .pane_layout_tabs(project_id)?
        .into_iter()
        .find_map(|tab| {
            let (tree, focused) = store.pane_layout(project_id, &tab).ok()??;
            let layout = crate::sessions::decode(project_id, &tree, &focused).ok()?;
            layout
                .tree
                .contains_leaf(pane_id)
                .then(|| (tab, layout.tree.leaves().len()))
        })
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that terminal is not in any tab"))?;
    let state = app.state::<crate::sessions::SessionState>();
    if panes > 1 {
        crate::arranging::session_close_leaf_now(
            app.clone(),
            &state,
            project_id.to_owned(),
            tab_id.clone(),
            pane_id.to_owned(),
        )?;
    } else {
        crate::arranging::session_close_tab_now(
            app.clone(),
            state,
            project_id.to_owned(),
            tab_id.clone(),
        )?;
        let _ = app.emit(
            TAB_CLOSED,
            json!({ "projectId": project_id, "tabId": tab_id }),
        );
    }
    Ok(tab_id)
}

/// Stops the sessions called `name` of this account — every one, since a
/// name started twice is one name to the person — and closes their terminals.
/// With `only`, the one process of that name; without, every one — a name
/// handed twice before names were unique is one piece of work.
pub(crate) fn stop(
    app: &AppHandle,
    profile_id: &str,
    name: &str,
    only: Option<i32>,
) -> Result<Value, RpcError> {
    let found: Vec<_> = crate::live_sessions::running_named(profile_id, name)?
        .into_iter()
        .filter(|one| only.is_none_or(|pid| one.pid == pid))
        .collect();
    if found.is_empty() {
        return Err(RpcError::new(
            ErrorCode::NotFound,
            format!("no session called {name} is running with that process"),
        ));
    }
    let mut terminal = Value::Null;
    for one in &found {
        match (&one.pane, &one.job) {
            (Some(pane), _) => {
                let tab_id = close_pane(app, &pane.project_id, &pane.pane_id)?;
                terminal = json!({ "projectId": pane.project_id, "tabId": tab_id });
            }
            (None, Some(job)) => stop_job(profile_id, job)
                .map_err(|why| RpcError::new(ErrorCode::Conflict, format!("{name}: {why}")))?,
            (None, None) => {
                // Outside devpit's terminals and not a job: only the process.
                if !devpit_pty::process::terminate(one.pid) {
                    return Err(RpcError::new(
                        ErrorCode::Conflict,
                        format!("{name} could not be stopped"),
                    ));
                }
            }
        }
    }
    let said = match (found.len(), terminal.is_null()) {
        (1, true) => format!("{name} was stopped"),
        (1, false) => format!("{name} was stopped, and its terminal closed"),
        (many, _) => format!("{many} sessions called {name} were stopped"),
    };
    Ok(json!({ "stopped": said, "terminal": terminal }))
}

/// A background job, stopped by its own CLI under the account that runs it.
fn stop_job(profile_id: &str, job: &str) -> Result<(), String> {
    let store = crate::projects::store().map_err(|err| err.to_string())?;
    let runner = crate::agent_profiles::runner_for(&store, profile_id)?;
    devpit_agentcli::stop_background(Some(&runner), job).map_err(|err| err.to_string())
}
