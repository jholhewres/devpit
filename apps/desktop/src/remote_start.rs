//! Work started from the Remote: a card, and a session on it or in the
//! project — the same doors the orchestrator uses, so the same checks hold.

use tauri::Emitter as _;

/// Puts a card in the project's first lane.
pub(crate) fn card(app: &tauri::AppHandle, project: &str, title: &str) -> Result<(), String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("a card needs a title".to_owned());
    }
    let board = crate::board::board_get_now(project.to_owned()).map_err(|err| err.message)?;
    let first = board.columns.first().ok_or("this board has no lane")?;
    crate::board::card_create_now(
        board.project_id.clone(),
        first.id.clone(),
        title.to_owned(),
        String::new(),
    )
    .map_err(|err| err.message)?;
    let _ = app.emit("board:changed", &board.project_id);
    Ok(())
}

/// The account a session from the Remote runs as: the default agent when it
/// is a Claude account, else the first Claude account there is.
pub(crate) fn profile(default: &str, profiles: &[devpit_rpc::Profile]) -> Option<String> {
    let claude = |one: &&devpit_rpc::Profile| one.driver == "claude" && one.enabled;
    profiles
        .iter()
        .filter(claude)
        .find(|one| one.id == default)
        .or_else(|| profiles.iter().find(claude))
        .map(|one| one.id.clone())
}

/// Starts a session in `project`, on `card` when there is one; answers its name.
pub(crate) fn session(
    app: &tauri::AppHandle,
    project: &str,
    card: Option<&str>,
    prompt: &str,
    anyway: bool,
) -> Result<String, String> {
    let store = crate::projects::store().map_err(|err| err.message)?;
    let profiles = crate::agent_profiles::all(&store).map_err(|err| err.message)?;
    let profile = self::profile(&crate::agent_choice::default_id(&store), &profiles)
        .ok_or("no Claude Code account is set up on this machine")?;
    let projects = crate::projects::project_list_unread_now()
        .map_err(|err| err.message)?
        .projects;
    let here = projects
        .iter()
        .find(|one| one.id == project)
        .ok_or("no such project")?;
    if !anyway {
        let needs = devpit_core::home::ProjectHome::of(
            &store,
            &devpit_core::Store::root().map_err(|err| err.to_string())?,
            project,
        )
        .ok()
        .map(|home| home.needs());
        let missing = crate::credentials::before_start(
            std::path::Path::new(&here.root_path),
            needs.as_deref(),
        );
        if !missing.is_empty() {
            return Err(
                crate::credentials::refused(&missing).replace("anyway: true", "Start anyway")
            );
        }
    }
    let answer = match card {
        Some(card) => {
            let board =
                crate::board::board_get_now(project.to_owned()).map_err(|err| err.message)?;
            crate::handing::hand(app, &board, &profile, card, prompt, None, None)?
        }
        None => {
            crate::opening::open(app, here, &profile, prompt, None).map_err(|err| err.message)?
        }
    };
    Ok(answer["name"].as_str().unwrap_or_default().to_owned())
}

#[cfg(test)]
#[path = "remote_start_tests.rs"]
mod tests;
