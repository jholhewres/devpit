//! An orchestrator's Schedules: the reminders it reaches — cards with a date,
//! in itself and its linked projects — and the routines the person set, in
//! one list.
//!
//! Only the person schedules. Routines and proposals are kept in the
//! orchestrator's settings, out of its chat's reach; what the orchestrator
//! proposes waits here until the person's click adds it.

use std::path::Path;

use devpit_core::Store;
use devpit_rpc::{
    ErrorCode, Project, ProposedReminder, Routine, RoutineKind, RpcError, Schedule, ScheduleKind,
    ScheduleProposal, ScheduleState, Schedules,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

/// The window hears this when a routine or a proposal changed.
pub(crate) const CHANGED: &str = "schedules:changed";
/// How long a reminder dealt with stays on the list.
const DONE_KEPT: i64 = 7 * 86_400;
/// The most proposals that wait at once.
const MOST_PROPOSED: usize = 20;
const ROUTINES: &str = "routines";
const PROPOSED: &str = "proposed";
/// The zone the window last said, for a routine the orchestrator proposes.
const ZONE: &str = "zone";

/// A routine as its orchestrator's settings keep it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Kept {
    pub id: String,
    #[serde(flatten)]
    pub routine: Routine,
    #[serde(default)]
    pub paused: bool,
    #[serde(default)]
    pub last_run: Option<i64>,
    #[serde(default)]
    pub next_at: Option<i64>,
}

pub(crate) fn title_of(kind: RoutineKind) -> &'static str {
    match kind {
        RoutineKind::Morning => "Morning summary",
        RoutineKind::Ci => "CI failures",
        RoutineKind::Prs => "Pull requests waiting",
        RoutineKind::Review => "Cards waiting in review",
    }
}

/// The first moment after `after` that `routine` runs: its minute, on one of
/// its days, at its offset.
pub(crate) fn next_run(routine: &Routine, after: i64) -> i64 {
    let offset = i64::from(routine.offset_minutes) * 60;
    let day = (after + offset).div_euclid(86_400);
    (0..=7)
        .map(|ahead| (day + ahead) * 86_400 + i64::from(routine.minute) * 60 - offset)
        .find(|&at| {
            let (weekday, _) = crate::channel_rules::local(at, routine.offset_minutes);
            at > after && (routine.days.is_empty() || routine.days.contains(&weekday))
        })
        .unwrap_or(after + 86_400)
}

/// Why `routine` cannot be kept, if it cannot.
fn invalid(routine: &Routine) -> Option<&'static str> {
    if routine.minute >= 24 * 60 {
        return Some("a routine runs at a time of the day");
    }
    if routine.days.iter().any(|day| *day > 6) {
        return Some("days go from 0, Sunday, to 6, Saturday");
    }
    if routine.offset_minutes.abs() > 14 * 60 {
        return Some("that is not a time zone");
    }
    None
}

fn read<T: serde::de::DeserializeOwned>(store: &Store, folder: &Path, key: &str) -> Vec<T> {
    store
        .orchestrator_settings(folder)
        .ok()
        .and_then(|all| serde_json::from_value(all[key].clone()).ok())
        .unwrap_or_default()
}

pub(crate) fn routines(store: &Store, folder: &Path) -> Vec<Kept> {
    read(store, folder, ROUTINES)
}

pub(crate) fn keep_routines(store: &Store, folder: &Path, all: &[Kept]) -> Result<(), RpcError> {
    Ok(store.set_orchestrator_setting(folder, ROUTINES, json!(all))?)
}

fn proposals(store: &Store, folder: &Path) -> Vec<ScheduleProposal> {
    read(store, folder, PROPOSED)
}

/// How many proposals wait for the person.
pub(crate) fn proposed_count(store: &Store, folder: &Path) -> usize {
    proposals(store, folder).len()
}

fn keep_proposals(store: &Store, folder: &Path, all: &[ScheduleProposal]) -> Result<(), RpcError> {
    Ok(store.set_orchestrator_setting(folder, PROPOSED, json!(all))?)
}

/// Every reminder `here` reaches and every routine it keeps, soonest first,
/// and what it proposed.
pub(crate) fn listed(store: &Store, here: &Project, projects: &[Project], now: i64) -> Schedules {
    let folder = Path::new(&here.root_path);
    let linked = crate::orchestrator_links::linked_in(store, folder);
    let name = |id: &str| {
        projects
            .iter()
            .find(|one| one.id == id)
            .map(|one| one.name.clone())
    };
    let reminders = store
        .reminders_listed(now - DONE_KEPT)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.project_id == here.id || linked.contains(&row.project_id))
        .map(|row| Schedule {
            kind: ScheduleKind::Reminder,
            at: Some(row.due_at as f64),
            project: name(&row.project_id),
            card_number: row.number,
            routine: None,
            state: match (row.handled_at, row.reminded_at) {
                (Some(_), _) => ScheduleState::Done,
                (None, Some(_)) => ScheduleState::Due,
                (None, None) => ScheduleState::Waiting,
            },
            last_run: row.reminded_at.map(|at| at as f64),
            id: row.card_id,
            title: row.title,
            project_id: Some(row.project_id),
        });
    let routines = routines(store, folder).into_iter().map(|kept| Schedule {
        id: kept.id,
        kind: ScheduleKind::Routine,
        title: title_of(kept.routine.kind).to_owned(),
        at: kept.next_at.map(|at| at as f64),
        project_id: Some(here.id.clone()),
        project: Some(here.name.clone()),
        card_number: None,
        state: if kept.paused {
            ScheduleState::Paused
        } else {
            ScheduleState::Waiting
        },
        last_run: kept.last_run.map(|at| at as f64),
        routine: Some(kept.routine),
    });
    let mut schedules: Vec<Schedule> = routines.chain(reminders).collect();
    schedules.sort_by(|a, b| {
        a.at.unwrap_or(f64::MAX)
            .total_cmp(&b.at.unwrap_or(f64::MAX))
    });
    Schedules {
        schedules,
        proposed: proposals(store, folder),
    }
}

/// A routine kept, new or changed: its next run counted from `now`.
pub(crate) fn saved(
    all: &mut Vec<Kept>,
    id: Option<&str>,
    routine: Routine,
    now: i64,
) -> Result<String, RpcError> {
    if let Some(why) = invalid(&routine) {
        return Err(RpcError::new(ErrorCode::Invalid, why));
    }
    let next_at = Some(next_run(&routine, now));
    if let Some(id) = id {
        let kept = all
            .iter_mut()
            .find(|one| one.id == id)
            .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such routine"))?;
        kept.routine = routine;
        kept.next_at = next_at;
        return Ok(id.to_owned());
    }
    let id = format!("routine_{}", ulid::Ulid::generate());
    all.push(Kept {
        id: id.clone(),
        routine,
        paused: false,
        last_run: None,
        next_at,
    });
    Ok(id)
}

/// Paused, a routine does not run; resumed, it runs next at its next time
/// from `now`, not at the ones it missed.
pub(crate) fn paused(all: &mut [Kept], id: &str, paused: bool, now: i64) -> Result<(), RpcError> {
    let kept = all
        .iter_mut()
        .find(|one| one.id == id)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such routine"))?;
    kept.paused = paused;
    if !paused {
        kept.next_at = Some(next_run(&kept.routine, now));
    }
    Ok(())
}

/// The window's zone, as it said it: kept for what the orchestrator proposes,
/// and given to the routines set in that zone when its offset moved.
fn seen_zone(
    store: &Store,
    folder: &Path,
    zone: &str,
    offset: i32,
    now: i64,
) -> Result<(), RpcError> {
    store.set_orchestrator_setting(
        folder,
        ZONE,
        json!({ "zone": zone, "offsetMinutes": offset }),
    )?;
    let mut all = routines(store, folder);
    let mut moved = false;
    for kept in all
        .iter_mut()
        .filter(|one| one.routine.zone == zone && one.routine.offset_minutes != offset)
    {
        kept.routine.offset_minutes = offset;
        kept.next_at = Some(next_run(&kept.routine, now));
        moved = true;
    }
    if moved {
        keep_routines(store, folder, &all)?;
    }
    Ok(())
}

/// A card with `title` due at `at`, in the first lane of `project`.
fn remind(project: &Project, title: &str, at: i64) -> Result<(), RpcError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "a reminder needs a title",
        ));
    }
    let board = crate::board::board_get_now(project.id.clone())?;
    let column = board
        .columns
        .iter()
        .min_by_key(|column| column.position)
        .ok_or_else(|| RpcError::new(ErrorCode::Invalid, "that board has no columns"))?;
    let card = crate::board::card_create_now(
        project.id.clone(),
        column.id.clone(),
        title.to_owned(),
        String::new(),
    )?;
    crate::cards::card_set_due_now(project.id.clone(), card.id, Some(at as f64), true)?;
    Ok(())
}

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs() as i64)
        .unwrap_or_default()
}

/// The orchestrator `project_id`, its folder and every project, for a command.
fn opened(project_id: &str) -> Result<(Store, Project, Vec<Project>), RpcError> {
    let store = crate::projects::store()?;
    crate::assistant::folder_of(&store, project_id)?;
    let projects = crate::projects::project_list_plain_now()?.projects;
    let here = projects
        .iter()
        .find(|one| one.id == project_id)
        .cloned()
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such orchestrator"))?;
    Ok((store, here, projects))
}

/// A routine changed: the window reads the list again.
fn told(app: &AppHandle, project_id: &str) {
    let _ = app.emit(CHANGED, project_id);
}

/// `schedules.list` — the orchestrator's Schedules, as the window in `zone`,
/// `offset_minutes` from UTC, reads them.
#[tauri::command]
#[specta::specta]
pub async fn schedules_list(
    project_id: String,
    zone: String,
    offset_minutes: i32,
) -> Result<Schedules, RpcError> {
    crate::off_main::blocking(move || {
        let (store, here, projects) = opened(&project_id)?;
        let folder = Path::new(&here.root_path);
        seen_zone(&store, folder, zone.trim(), offset_minutes, now())?;
        Ok(listed(&store, &here, &projects, now()))
    })
    .await
}

/// `schedule.routine_save` — a routine the person set, new or changed.
#[tauri::command]
#[specta::specta]
pub async fn schedule_routine_save(
    app: AppHandle,
    project_id: String,
    id: Option<String>,
    routine: Routine,
) -> Result<Schedules, RpcError> {
    crate::off_main::blocking(move || {
        let (store, here, projects) = opened(&project_id)?;
        let folder = Path::new(&here.root_path);
        let mut all = routines(&store, folder);
        saved(&mut all, id.as_deref(), routine, now())?;
        keep_routines(&store, folder, &all)?;
        told(&app, &project_id);
        Ok(listed(&store, &here, &projects, now()))
    })
    .await
}

/// `schedule.routine_pause` — a routine paused or resumed.
#[tauri::command]
#[specta::specta]
pub async fn schedule_routine_pause(
    app: AppHandle,
    project_id: String,
    id: String,
    pause: bool,
) -> Result<Schedules, RpcError> {
    crate::off_main::blocking(move || {
        let (store, here, projects) = opened(&project_id)?;
        let folder = Path::new(&here.root_path);
        let mut all = routines(&store, folder);
        paused(&mut all, &id, pause, now())?;
        keep_routines(&store, folder, &all)?;
        told(&app, &project_id);
        Ok(listed(&store, &here, &projects, now()))
    })
    .await
}

/// `schedule.routine_delete` — a routine gone.
#[tauri::command]
#[specta::specta]
pub async fn schedule_routine_delete(
    app: AppHandle,
    project_id: String,
    id: String,
) -> Result<Schedules, RpcError> {
    crate::off_main::blocking(move || {
        let (store, here, projects) = opened(&project_id)?;
        let folder = Path::new(&here.root_path);
        let mut all = routines(&store, folder);
        all.retain(|one| one.id != id);
        keep_routines(&store, folder, &all)?;
        told(&app, &project_id);
        Ok(listed(&store, &here, &projects, now()))
    })
    .await
}

/// `schedule.remind` — a reminder the person made by hand, on `target`: the
/// orchestrator itself or a project linked to it.
#[tauri::command]
#[specta::specta]
pub async fn schedule_remind(
    app: AppHandle,
    project_id: String,
    target: String,
    title: String,
    at: f64,
) -> Result<Schedules, RpcError> {
    crate::off_main::blocking(move || {
        let (store, here, projects) = opened(&project_id)?;
        let on = reached(&store, &here, &projects, &target)?;
        if !at.is_finite() || (at as i64) <= now() {
            return Err(RpcError::new(ErrorCode::Invalid, "a reminder is for later"));
        }
        remind(on, &title, at as i64)?;
        crate::reminders::changed(&app);
        Ok(listed(&store, &here, &projects, now()))
    })
    .await
}

/// `schedule.proposal` — the person adds what the orchestrator proposed, as
/// the window in `zone` sees it, or drops it.
#[tauri::command]
#[specta::specta]
pub async fn schedule_proposal(
    app: AppHandle,
    project_id: String,
    id: String,
    add: bool,
    zone: String,
    offset_minutes: i32,
) -> Result<Schedules, RpcError> {
    crate::off_main::blocking(move || {
        let (store, here, projects) = opened(&project_id)?;
        let seen = (zone.as_str(), offset_minutes);
        if decided(&store, &here, &projects, &id, add, seen, now())? {
            crate::reminders::changed(&app);
        }
        told(&app, &project_id);
        Ok(listed(&store, &here, &projects, now()))
    })
    .await
}

/// The proposal `id` taken off the list and, when `add`, made the person's
/// own, in the window's zone. Answers whether a reminder was made.
pub(crate) fn decided(
    store: &Store,
    here: &Project,
    projects: &[Project],
    id: &str,
    add: bool,
    (zone, offset): (&str, i32),
    now: i64,
) -> Result<bool, RpcError> {
    let folder = Path::new(&here.root_path);
    let mut waiting = proposals(store, folder);
    let at = waiting
        .iter()
        .position(|one| one.id == id)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "that proposal is gone"))?;
    let proposal = waiting.remove(at);
    let mut reminded = false;
    if add {
        if let Some(mut routine) = proposal.routine {
            zone.clone_into(&mut routine.zone);
            routine.offset_minutes = offset;
            let mut all = routines(store, folder);
            saved(&mut all, None, routine, now)?;
            keep_routines(store, folder, &all)?;
        }
        if let Some(reminder) = proposal.reminder {
            let on = reached(store, here, projects, &reminder.project_id)?;
            remind(on, &reminder.title, reminder.at as i64)?;
            reminded = true;
        }
    }
    keep_proposals(store, folder, &waiting)?;
    Ok(reminded)
}

/// The project a reminder goes on: the orchestrator, or one linked to it.
fn reached<'a>(
    store: &Store,
    here: &'a Project,
    projects: &'a [Project],
    target: &str,
) -> Result<&'a Project, RpcError> {
    if target == here.id {
        return Ok(here);
    }
    let linked = crate::orchestrator_links::linked_in(store, Path::new(&here.root_path));
    projects
        .iter()
        .find(|one| one.id == target && linked.contains(&one.id))
        .ok_or_else(|| RpcError::new(ErrorCode::Invalid, "that project is not linked here"))
}

/// What an orchestrator in Assistant mode asks: its Schedules, or a proposal
/// for the person.
pub(crate) fn respond(
    app: Option<&AppHandle>,
    method: &str,
    here: &Project,
    project: &Project,
    projects: &[Project],
    params: &Value,
) -> Result<Value, String> {
    let store = crate::projects::store().map_err(|err| err.message)?;
    let folder = Path::new(&here.root_path);
    match method {
        "schedules" => Ok(json!(listed(&store, here, projects, now()))),
        "schedule_propose" => {
            let proposal = proposed(&store, folder, project, params, now())?;
            let mut waiting = proposals(&store, folder);
            if waiting.len() >= MOST_PROPOSED {
                return Err(format!(
                    "{MOST_PROPOSED} proposals already wait for the person: let them decide first"
                ));
            }
            waiting.push(proposal.clone());
            keep_proposals(&store, folder, &waiting).map_err(|err| err.message)?;
            if let Some(app) = app {
                told(app, &here.id);
            }
            Ok(json!({
                "proposal": proposal,
                "note": "Nothing is scheduled: it waits in the person's Schedules, and only their click adds it. Tell them so.",
            }))
        }
        _ => Err(format!("devpit does not answer `{method}`")),
    }
}

/// A proposal from what the orchestrator passed: a routine at a time in the
/// zone the window last said, or a reminder on `project`.
pub(crate) fn proposed(
    store: &Store,
    folder: &Path,
    project: &Project,
    params: &Value,
    now: i64,
) -> Result<ScheduleProposal, String> {
    let text = |name: &str| params.get(name).and_then(Value::as_str).map(str::trim);
    let why = text("why")
        .filter(|why| !why.is_empty())
        .ok_or("say why in `why`: the person reads it beside the proposal")?;
    let id = format!("proposal_{}", ulid::Ulid::generate());
    if let Some(kind) = text("routine") {
        let kind: RoutineKind = serde_json::from_value(json!(kind))
            .map_err(|_| format!("`{kind}` is not a routine: morning, ci, prs or review"))?;
        let time = text("time").ok_or("a routine needs `time`, as HH:MM")?;
        let minute = time
            .split_once(':')
            .and_then(|(h, m)| Some(h.parse::<u32>().ok()? * 60 + m.parse::<u32>().ok()?))
            .filter(|_| time.len() == 5)
            .ok_or_else(|| format!("`{time}` is not a time: write it as HH:MM"))?;
        let days = params
            .get("days")
            .and_then(Value::as_array)
            .map(|days| {
                days.iter()
                    .filter_map(Value::as_u64)
                    .map(|day| day as u32)
                    .collect()
            })
            .unwrap_or_default();
        let seen = store
            .orchestrator_settings(folder)
            .map(|all| all[ZONE].clone())
            .unwrap_or_default();
        let routine = Routine {
            kind,
            minute,
            days,
            zone: seen["zone"].as_str().unwrap_or("UTC").to_owned(),
            offset_minutes: seen["offsetMinutes"].as_i64().unwrap_or(0) as i32,
        };
        if let Some(why) = invalid(&routine) {
            return Err(why.to_owned());
        }
        return Ok(ScheduleProposal {
            id,
            routine: Some(routine),
            reminder: None,
            why: why.to_owned(),
        });
    }
    let at = crate::reminder_time::instant_of(
        text("at").ok_or("propose a routine (`routine`, `time`) or a reminder (`at`, `title`)")?,
    )?;
    if at <= now {
        return Err("that moment has passed: a reminder is for later".to_owned());
    }
    let title = text("title")
        .filter(|title| !title.is_empty())
        .ok_or("a reminder needs a `title`")?;
    Ok(ScheduleProposal {
        id,
        routine: None,
        reminder: Some(ProposedReminder {
            title: title.to_owned(),
            at: at as f64,
            project_id: project.id.clone(),
            project: Some(project.name.clone()),
        }),
        why: why.to_owned(),
    })
}

#[cfg(test)]
#[path = "schedules_tests.rs"]
mod tests;
