//! A session started by an orchestrator in a project's own folder, with no
//! card: a new terminal tab of that project, the account's Claude Code typed
//! into it with the name and the brief. It is visible like any tab — opened
//! over the chat, gone to, stopped — rather than running where nobody looks.

use devpit_rpc::{ErrorCode, Project, RpcError};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

/// Said when a tab was opened from here, so the window lists it for its project.
pub(crate) const TAB_OPENED: &str = "session:tab-opened";

/// The longest brief typed: a brief, not a document.
const LONGEST: usize = 8000;

pub(crate) fn open(
    app: &AppHandle,
    project: &Project,
    profile_id: &str,
    prompt: &str,
    named: Option<&str>,
) -> Result<Value, RpcError> {
    let prompt = prompt.trim();
    if prompt.is_empty() || prompt.chars().count() > LONGEST {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!("a session is started with between 1 and {LONGEST} characters of brief"),
        ));
    }
    let name = named
        .map(str::trim)
        .filter(|one| !one.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| {
            let short: String = ulid::Ulid::generate()
                .to_string()
                .chars()
                .rev()
                .take(4)
                .collect();
            format!(
                "{}-{}",
                crate::chat_remote::remote_name(&project.name).trim_start_matches("devpit-"),
                short.to_lowercase()
            )
        });
    let tab_id = fresh_tab();
    let pane_id = typed_in(
        app,
        &project.id,
        &tab_id,
        std::path::Path::new(&project.root_path),
        &launched(profile_id, &name, prompt)?,
    )?;
    let _ = app.emit(
        TAB_OPENED,
        json!({ "projectId": project.id, "tabId": tab_id, "paneId": pane_id }),
    );
    Ok(json!({
        "name": name,
        "project": project.name,
        "cwd": project.root_path,
        "next": "It runs in a new terminal tab of that project. Message it by this name with SendMessage once it is up, and pass notify_when_idle to hear when it is done.",
    }))
}

/// A tab id nothing else has.
pub(crate) fn fresh_tab() -> String {
    format!("tab_{}", ulid::Ulid::generate().to_string().to_lowercase())
}

/// The account's Claude Code, named and briefed, as one line to type.
pub(crate) fn launched(profile_id: &str, name: &str, prompt: &str) -> Result<String, RpcError> {
    Ok(format!(
        "{} --name {} {}",
        crate::shell_launch::to_start(profile_id)?,
        devpit_agentcli::declaring::quoted(name),
        devpit_agentcli::declaring::quoted(prompt)
    ))
}

/// Opens tab `tab_id` of the project with its pane in `cwd`, and types `line`
/// at its prompt. Answers the pane.
pub(crate) fn typed_in(
    app: &AppHandle,
    project_id: &str,
    tab_id: &str,
    cwd: &std::path::Path,
    line: &str,
) -> Result<String, RpcError> {
    let state = app.state::<crate::sessions::SessionState>();
    let layout = crate::sessions::ensure_at(&state, project_id, tab_id, cwd)?;
    let session = devpit_tmux::Server::session_name(project_id);
    let target = devpit_tmux::Server::target(&session, &layout.focused_id);
    let _ = crate::shell_launch::settled(&session, &layout.focused_id);
    crate::sessions::tmux_server()?
        .send_keys(&target, line)
        .map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(layout.focused_id)
}

/// `session.watch` — a background session, attached in a new terminal tab of
/// its project: it ran under the CLI's own supervisor, where nobody could see
/// it, and there it is watched and typed into like any other.
#[tauri::command]
#[specta::specta]
pub async fn session_watch(
    app: AppHandle,
    profile_id: String,
    project_id: String,
    job: String,
) -> Result<String, RpcError> {
    crate::off_main::blocking(move || {
        if !crate::adopting::plain(&job) {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "that is not a session's job id",
            ));
        }
        let store = crate::projects::store()?;
        let runner = crate::agent_profiles::runner_for(&store, &profile_id)
            .map_err(|why| RpcError::new(ErrorCode::NotFound, why))?;
        let root = crate::roots::root_of(&project_id, None)?;
        let line = format!(
            "{} attach {}",
            devpit_agentcli::running::line(&runner),
            devpit_agentcli::declaring::quoted(&job)
        );
        let tab_id = fresh_tab();
        let pane_id = typed_in(&app, &project_id, &tab_id, &root, &line)?;
        let _ = app.emit(
            TAB_OPENED,
            json!({ "projectId": project_id, "tabId": tab_id, "paneId": pane_id }),
        );
        Ok(pane_id)
    })
    .await
}
