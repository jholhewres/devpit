//! Autopilot, on this machine: once the person approved a plan and started
//! it, the epic's cards run in the order their blockers allow, one session
//! per card through devpit's own launch (`card_follows::start` and
//! `handing::hand`, which hands the session the card's last handoff note).
//! Each card passes the gate before it finishes; the next starts when one
//! finishes or the person moves one, and nothing runs on a timer.
//!
//! The order and the stops are `devpit_devflow::autopilot`'s. An agent can
//! only ask for it: the person's click starts it. It is held in memory, so a
//! restart stops it, and the board says where it stood.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use devpit_core::Store;
use devpit_devflow::autopilot::{self, Next, Planned, Standing};
use devpit_devflow::config::{Approve, Config};
use devpit_rpc::{Board, DevflowAutopilot, DevflowChange, Doing, ErrorCode, Project, RpcError};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

/// The window reads the autopilot again: its payload is the project's id.
pub(crate) const MOVED: &str = "devflow:autopilot";

/// One autopilot: the epic, the agent profile its sessions start as, and
/// what it last said.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Running {
    epic: String,
    profile: String,
    said: String,
}

fn running() -> &'static Mutex<HashMap<String, Running>> {
    static RUNNING: OnceLock<Mutex<HashMap<String, Running>>> = OnceLock::new();
    RUNNING.get_or_init(Mutex::default)
}

/// How a session starts on a card: devpit's own launch, or a test's.
pub(crate) trait Sessions {
    fn start(&self, board: &Board, card_id: &str, prompt: &str) -> Result<(), String>;
}

/// The epic's cards as the scheduler reads them, by their lanes and what
/// their sessions add up to.
pub(crate) fn planned(board: &Board, epic: &str) -> Vec<Planned> {
    let role = |column: &str| {
        board
            .columns
            .iter()
            .find(|one| one.id == column)
            .and_then(|one| one.role.as_deref())
    };
    board
        .cards
        .iter()
        .filter(|card| card.parent_id.as_deref() == Some(epic))
        .map(|card| Planned {
            id: card.id.clone(),
            title: card.title.clone(),
            blocked_by: card
                .blocked_by
                .iter()
                .map(|by| (by.id.clone(), by.done))
                .collect(),
            standing: match role(&card.column_id) {
                Some("done") => Standing::Done,
                Some("check") => Standing::Finished,
                Some("doing") => match card.activity {
                    Some(Doing::Waiting) => Standing::Stuck(format!(
                        "\"{}\" waits for your answer in its session",
                        card.title
                    )),
                    Some(Doing::Failed | Doing::Gone) => Standing::Stuck(format!(
                        "\"{}\"'s session ended before the card passed its gate",
                        card.title
                    )),
                    _ => Standing::Working,
                },
                _ => Standing::Waiting,
            },
        })
        .collect()
}

/// Whether the flow asks the person to approve each card once it is done:
/// an own stage after the plan with `approve: person`.
pub(crate) fn approve_each(config: &Config) -> bool {
    let after_plan = config
        .flow
        .iter()
        .skip_while(|step| step.builtin() != Some("plan"))
        .skip(1);
    after_plan
        .filter(|step| config.is_on(step))
        .any(|step| step.own.as_ref().and_then(|own| own.approve) == Some(Approve::Person))
}

/// What a card's session is handed. The handoff note is added by the launch.
fn prompt(epic: &str, card: &Planned) -> String {
    format!(
        "Autopilot of the epic \"{epic}\": work on the card \"{}\" ({}) through Devflow. \
         Follow devflow-execution (devpit_devflow_skill), run devpit_devflow_evidence, the \
         review and any own stage (devpit_devflow_stage), then devpit_devflow_gate. When it \
         passes, write the handoff with devpit_handoff and call devpit_finish_card. Stop and \
         ask the person before any commit, push or deploy, and when the gate keeps failing \
         for a reason you cannot fix.",
        card.title, card.id
    )
}

/// One step of an autopilot: starts the next card, or says why it waits.
pub(crate) fn stepped(
    board: &Board,
    epic: &str,
    approve: bool,
    sessions: &dyn Sessions,
) -> Result<(Next, String), String> {
    let cards = planned(board, epic);
    let title = board
        .cards
        .iter()
        .find(|card| card.id == epic)
        .map_or(epic.to_owned(), |card| card.title.clone());
    let next = autopilot::next(&cards, approve);
    let said = match &next {
        Next::Start(id) => {
            let card = cards
                .iter()
                .find(|card| card.id == *id)
                .ok_or("the card is gone")?;
            sessions.start(board, id, &prompt(&title, card))?;
            format!("started \"{}\"", card.title)
        }
        Next::Working(id) => format!(
            "\"{}\" is at work",
            cards
                .iter()
                .find(|card| card.id == *id)
                .map_or("a card", |card| card.title.as_str())
        ),
        Next::Approve { why, .. } => format!("waits for you: {why}"),
        Next::Blocked { why, .. } => format!("stopped: {why}"),
        Next::Done => "done: every card passed its gate".to_owned(),
    };
    Ok((next, said))
}

/// devpit's own launch: the card to work in progress, then a session in its
/// own checkout and terminal, as an orchestrator starts one.
struct Launched<'a> {
    app: &'a AppHandle,
    profile: String,
}

impl Sessions for Launched<'_> {
    fn start(&self, board: &Board, card_id: &str, prompt: &str) -> Result<(), String> {
        crate::card_follows::start(Some(self.app), board, card_id)?;
        crate::handing::hand(self.app, board, &self.profile, card_id, prompt, None, None)?;
        Ok(())
    }
}

/// The agent profile a new terminal opens, else the first one switched on.
fn profile(store: &Store) -> Result<String, String> {
    let all = crate::agent_profiles::all(store).map_err(|err| err.message)?;
    let default = crate::agent_choice::default_id(store);
    all.iter()
        .find(|one| one.id == default && one.enabled)
        .or_else(|| all.iter().find(|one| one.enabled))
        .map(|one| one.id.clone())
        .ok_or_else(|| "no agent is set up on this machine to run the cards".to_owned())
}

fn board_and_config(project_id: &str) -> Result<(Board, Config), String> {
    let board = crate::board::board_get_now(project_id.to_owned()).map_err(|err| err.message)?;
    let store = crate::projects::store().map_err(|err| err.message)?;
    let (_, root) = crate::projects::locate(&store, project_id).map_err(|err| err.message)?;
    match crate::devflow::read(&root) {
        devpit_devflow::State::Valid(_, config) => Ok((board, *config)),
        _ => Err("devflow.yaml does not read: the Devflow screen says why".to_owned()),
    }
}

/// Moves the project's autopilot on, when one runs.
pub(crate) fn advance(app: &AppHandle, project_id: &str) -> Result<(), String> {
    let Some(run) = running()
        .lock()
        .ok()
        .and_then(|held| held.get(project_id).cloned())
    else {
        return Ok(());
    };
    let (board, config) = board_and_config(project_id)?;
    let launched = Launched {
        app,
        profile: run.profile.clone(),
    };
    let (next, said) = match stepped(&board, &run.epic, approve_each(&config), &launched) {
        Ok(stepped) => stepped,
        Err(why) => (
            Next::Blocked {
                card: String::new(),
                why: why.clone(),
            },
            format!("stopped: {why}"),
        ),
    };
    if let Ok(mut held) = running().lock() {
        match next {
            Next::Done | Next::Blocked { .. } => {
                held.remove(project_id);
            }
            _ => {
                if let Some(one) = held.get_mut(project_id) {
                    one.said = said.clone();
                }
            }
        }
        last_said()
            .lock()
            .map(|mut last| last.insert(project_id.to_owned(), (run.epic.clone(), said)))
            .ok();
    }
    let _ = app.emit(MOVED, project_id);
    let _ = app.emit("board:changed", project_id);
    Ok(())
}

/// What a stopped autopilot said last, for the epic to show.
fn last_said() -> &'static Mutex<HashMap<String, (String, String)>> {
    static SAID: OnceLock<Mutex<HashMap<String, (String, String)>>> = OnceLock::new();
    SAID.get_or_init(Mutex::default)
}

/// A card finished or moved: the project's autopilot looks again, on its
/// own thread — the caller is answering an agent or the window.
pub(crate) fn heard(app: Option<&AppHandle>, project_id: &str) {
    let Some(app) = app.cloned() else {
        return;
    };
    let runs = running()
        .lock()
        .is_ok_and(|held| held.contains_key(project_id));
    if !runs {
        return;
    }
    let project_id = project_id.to_owned();
    std::thread::spawn(move || {
        let _ = advance(&app, &project_id);
    });
}

/// The person's click: the epic's autopilot starts, and its first card with it.
pub(crate) fn start(app: &AppHandle, project_id: &str, epic: &str) -> Result<(), String> {
    let store = crate::projects::store().map_err(|err| err.message)?;
    let (board, _) = board_and_config(project_id)?;
    if planned(&board, epic).is_empty() {
        return Err("the epic has no cards yet: create its plan first".to_owned());
    }
    if !crate::devflow::is_on(
        project_id,
        &crate::projects::locate(&store, project_id)
            .map_err(|err| err.message)?
            .1,
    ) {
        return Err("Devflow is not on in this project".to_owned());
    }
    let profile = profile(&store)?;
    running()
        .lock()
        .map_err(|_| "the autopilot could not start")?
        .insert(
            project_id.to_owned(),
            Running {
                epic: epic.to_owned(),
                profile,
                said: "starting".to_owned(),
            },
        );
    advance(app, project_id)
}

fn read_now(project_id: &str) -> DevflowAutopilot {
    let run = running()
        .lock()
        .ok()
        .and_then(|held| held.get(project_id).cloned());
    let last = last_said()
        .lock()
        .ok()
        .and_then(|said| said.get(project_id).cloned());
    let epic = run
        .as_ref()
        .map(|one| one.epic.clone())
        .or(last.as_ref().map(|(epic, _)| epic.clone()));
    let order = epic
        .as_deref()
        .and_then(|epic| {
            let (board, _) = board_and_config(project_id).ok()?;
            let cards = planned(&board, epic);
            let order = autopilot::ordered(&cards).ok()?;
            Some(order.iter().map(|card| card.title.clone()).collect())
        })
        .unwrap_or_default();
    DevflowAutopilot {
        project_id: project_id.to_owned(),
        running: run.is_some(),
        said: run.map(|one| one.said).or(last.map(|(_, said)| said)),
        epic_id: epic,
        order,
    }
}

/// `devflow_autopilot`, asked by an agent: the order the epic's cards would
/// run in and where it would stop. The agent cannot start it — devpit asks
/// the person, whose click does.
pub(crate) fn respond(
    app: Option<&AppHandle>,
    store: &Store,
    project: &Project,
    params: &Value,
) -> Result<Value, String> {
    let epic = params
        .get("epic")
        .and_then(Value::as_str)
        .ok_or("pass the epic's card id as `epic`")?;
    let board = crate::board::board_get_now(project.id.clone()).map_err(|err| err.message)?;
    let cards = planned(&board, epic);
    if cards.is_empty() {
        return Err("that epic has no cards: its plan is created first, on the board".to_owned());
    }
    let order = autopilot::ordered(&cards)?;
    let titles: Vec<&str> = order.iter().map(|card| card.title.as_str()).collect();
    let _ = store;
    let epic_title = board
        .cards
        .iter()
        .find(|card| card.id == epic)
        .map_or(epic, |card| card.title.as_str());
    crate::devflow_asks::raise_for(
        app,
        &project.id,
        &project.name,
        DevflowChange::Autopilot,
        &format!("{epic_title}: {}", titles.join(" → ")),
        epic,
    );
    Ok(json!({
        "order": titles,
        "running": read_now(&project.id).running,
        "note": "Nothing started. devpit asks the person to start the autopilot in the app; it runs one card at a time and stops at a card others wait on until the person checked it, at a flow's approve: person, and at a session waiting on them.",
    }))
}

/// `devflow.autopilot` — the epic's autopilot as it stands.
#[tauri::command]
#[specta::specta]
pub async fn devflow_autopilot(project_id: String) -> Result<DevflowAutopilot, RpcError> {
    crate::off_main::blocking(move || Ok(read_now(&project_id))).await
}

/// `devflow.autopilot_start` — the person's click on the epic: its cards run.
#[tauri::command]
#[specta::specta]
pub async fn devflow_autopilot_start(
    app: AppHandle,
    project_id: String,
    epic_id: String,
) -> Result<DevflowAutopilot, RpcError> {
    crate::off_main::blocking(move || {
        start(&app, &project_id, &epic_id).map_err(|why| RpcError::new(ErrorCode::Invalid, why))?;
        Ok(read_now(&project_id))
    })
    .await
}

/// `devflow.autopilot_stop` — the person stops it; the card at work goes on
/// in its own session, and nothing starts after it.
#[tauri::command]
#[specta::specta]
pub async fn devflow_autopilot_stop(
    app: AppHandle,
    project_id: String,
) -> Result<DevflowAutopilot, RpcError> {
    crate::off_main::blocking(move || {
        if let Ok(mut held) = running().lock() {
            if let Some(run) = held.remove(&project_id) {
                last_said()
                    .lock()
                    .map(|mut said| {
                        said.insert(project_id.clone(), (run.epic, "stopped by you".to_owned()))
                    })
                    .ok();
            }
        }
        let _ = app.emit(MOVED, &project_id);
        Ok(read_now(&project_id))
    })
    .await
}

#[cfg(test)]
#[path = "devflow_autopilot_tests.rs"]
mod tests;
