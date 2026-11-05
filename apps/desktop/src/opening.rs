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
    let name = crate::handing::unused(
        named
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
            }),
        &crate::handing::live_names(profile_id),
    );
    let tab_id = fresh_tab();
    let pane_id = typed_in(
        app,
        &project.id,
        &tab_id,
        std::path::Path::new(&project.root_path),
        &launched(profile_id, &project.id, &name, prompt)?,
        Some(profile_id),
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

/// Whether text can be typed at a shell inside quotes and stay text.
///
/// A control character is a key: Ctrl-C or Ctrl-U in a brief would end the
/// quoted argument at the terminal and run what follows it as a command.
/// Lines and tabs stay inside the quotes, so they are kept.
pub(crate) fn typeable(text: &str) -> Result<(), RpcError> {
    match text
        .chars()
        .find(|ch| ch.is_control() && *ch != '\n' && *ch != '\t')
    {
        Some(ch) => Err(RpcError::new(
            ErrorCode::Invalid,
            format!(
                "a brief cannot hold control characters (found U+{:04X})",
                u32::from(ch)
            ),
        )),
        None => Ok(()),
    }
}

/// The account's Claude Code, named and briefed, as one line to type.
pub(crate) fn launched(
    profile_id: &str,
    project_id: &str,
    name: &str,
    prompt: &str,
) -> Result<String, RpcError> {
    typeable(name)?;
    typeable(prompt)?;
    Ok(format!(
        "{} --name {} {}",
        crate::shell_launch::to_start_in(profile_id, Some(project_id))?,
        devpit_agentcli::declaring::quoted(name),
        devpit_agentcli::declaring::quoted(prompt)
    ))
}

/// Opens tab `tab_id` of the project with its pane in `cwd`, and types `line`
/// at its prompt. Answers the pane. `launch` is the profile `line` starts, kept
/// on the pane; none for a line that starts no agent of its own.
pub(crate) fn typed_in(
    app: &AppHandle,
    project_id: &str,
    tab_id: &str,
    cwd: &std::path::Path,
    line: &str,
    launch: Option<&str>,
) -> Result<String, RpcError> {
    let state = app.state::<crate::sessions::SessionState>();
    let layout = crate::sessions::ensure_at(&state, project_id, tab_id, cwd)?;
    let target = crate::pane_io::target_in(&layout, &layout.focused_id);
    let _ = crate::pane_io::settled(&target);
    // Before the CLI looks: a session stopped on the trust question is not up.
    crate::folder_trust::trust(&[cwd]);
    match launch {
        Some(launch) => type_agent(
            crate::sessions::store().ok().as_ref(),
            None,
            project_id,
            &layout.focused_id,
            &target,
            line,
            launch,
        )?,
        None => crate::pane_io::send_line(&target, line)?,
    }
    crate::trust_answer::watch(target);
    Ok(layout.focused_id)
}

/// Types an agent's launch `line` at a pane and keeps the launch on the pane:
/// what its hooks tie the session to, and what a lost pane starts again.
/// `home` is a ptyd daemon's, when not this app's own.
pub(crate) fn type_agent(
    store: Option<&devpit_core::Store>,
    home: Option<&std::path::Path>,
    project_id: &str,
    leaf_id: &str,
    target: &str,
    line: &str,
    launch: &str,
) -> Result<(), RpcError> {
    crate::pane_io::send_line_at(home, target, line)?;
    if let Some(store) = store {
        let _ = store.remember_pane_launch(project_id, leaf_id, launch);
    }
    Ok(())
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
        // An attach starts no agent: the session runs under the CLI's supervisor.
        let pane_id = typed_in(&app, &project_id, &tab_id, &root, &line, None)?;
        let _ = app.emit(
            TAB_OPENED,
            json!({ "projectId": project_id, "tabId": tab_id, "paneId": pane_id }),
        );
        Ok(pane_id)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::typeable;

    /// Ctrl-C and Ctrl-U are keys at a terminal: inside a typed brief they end
    /// the quoted argument and run the rest as a command.
    #[test]
    fn a_brief_with_a_control_character_is_refused() {
        assert!(typeable("fix it\x03; rm -rf ~").is_err());
        assert!(typeable("line one\x15rm -rf ~").is_err());
        assert!(typeable("two lines\nand a\ttab").is_ok());
    }

    /// An orchestrator's session, typed into a ptyd pane where no `$TMUX`
    /// says where it runs: its first hook ties it to that pane.
    #[cfg(unix)]
    #[test]
    fn a_session_typed_into_a_ptyd_pane_is_found_there_once_it_starts() {
        use devpit_agentcli::{Event, Happening};

        let daemon = crate::ptyd_link::tests::serving();
        let home = daemon.home.path();
        let store = devpit_core::Store::open(&home.join("state.db")).expect("store");
        let project = store.add_project(home, None).expect("project");
        let leaf = "leaf_typed_session";
        let pane = crate::ptyd_link::pane_number(leaf);
        let asked = devpit_ptyd::pane::Asked {
            rows: 24,
            cols: 80,
            cwd: home.to_string_lossy().into_owned(),
            argv: vec!["/bin/sh".into(), "-c".into(), "read line; sleep 30".into()],
            env: Vec::new(),
        };
        crate::ptyd_link::spawn(home, pane, &asked).expect("spawn");
        let target = crate::ptyd_link::target(leaf);

        let typed = super::type_agent(
            Some(&store),
            Some(home),
            &project,
            leaf,
            &target,
            "claude --name typed 'the brief'",
            "claude",
        );
        let started = Happening {
            session_id: "8f0c2b7e-0d4b-4c55-9d7e-3b1a2c4d5e6f".into(),
            event: Event::SessionStarted,
            cwd: home.to_string_lossy().into_owned(),
            transcript_path: None,
        };
        crate::restoring::remember(&store, leaf, &started);
        crate::ptyd_link::kill(home, pane);

        typed.expect("typed");
        assert_eq!(
            store
                .place_of_session("8f0c2b7e-0d4b-4c55-9d7e-3b1a2c4d5e6f")
                .expect("read"),
            Some((project, leaf.to_owned()))
        );
    }
}
