use std::path::Path;

use devpit_core::Store;
use devpit_rpc::{Project, Routine, RoutineKind, ScheduleKind, ScheduleState};
use serde_json::json;

use super::{
    decided, keep_proposals, keep_routines, listed, next_run, paused, proposed, routines, saved,
};

/// Friday 2026-11-13, 00:00 UTC.
const FRIDAY: i64 = 1_794_528_000;
const HOUR: i64 = 3_600;

fn nine_in_sao_paulo(days: Vec<u32>) -> Routine {
    Routine {
        kind: RoutineKind::Morning,
        minute: 9 * 60,
        days,
        zone: "America/Sao_Paulo".to_owned(),
        offset_minutes: -180,
    }
}

fn project(id: &str, root: &Path, orchestrator: Option<&str>) -> Project {
    serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": root.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": orchestrator,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project")
}

/// A store with an orchestrator and one linked project, each with a card due.
fn store() -> (tempfile::TempDir, Store, Project, Vec<Project>) {
    let dir = tempfile::tempdir().expect("tmp");
    let store = Store::open(&dir.path().join("state.db")).expect("store");
    let mut made = Vec::new();
    let made_at = [("orch", Some("claude")), ("linked", None), ("other", None)];
    for (hours, (name, orchestrator)) in (20..).zip(made_at) {
        let root = dir.path().join(name);
        std::fs::create_dir_all(&root).expect("dir");
        let id = store.add_project(&root, None).expect("project");
        store.ensure_board(&id).expect("board");
        let column = store.columns(&id).expect("columns")[0].id.clone();
        let card = store
            .create_card(&id, &column, &format!("Call {name}"), "")
            .expect("card");
        store
            .set_card_due(&card, Some(FRIDAY + hours * HOUR), true)
            .expect("due");
        made.push(project(&id, &root, orchestrator));
    }
    let here = made[0].clone();
    store
        .set_orchestrator_setting(Path::new(&here.root_path), "projects", json!([made[1].id]))
        .expect("linked");
    (dir, store, here, made)
}

#[test]
fn a_routine_runs_at_its_time_in_its_zone_on_its_days() {
    // 09:00 in São Paulo is 12:00 UTC.
    let every_day = nine_in_sao_paulo(Vec::new());
    assert_eq!(next_run(&every_day, FRIDAY + 11 * HOUR), FRIDAY + 12 * HOUR);
    assert_eq!(
        next_run(&every_day, FRIDAY + 12 * HOUR),
        FRIDAY + 36 * HOUR,
        "not twice"
    );
    // On weekdays, Friday afternoon's next run is Monday morning.
    let weekdays = nine_in_sao_paulo(vec![1, 2, 3, 4, 5]);
    assert_eq!(
        next_run(&weekdays, FRIDAY + 13 * HOUR),
        FRIDAY + 3 * 24 * HOUR + 12 * HOUR
    );
}

#[test]
fn a_paused_routine_resumes_at_its_next_time_not_at_the_ones_it_missed() {
    let mut all = Vec::new();
    let id = saved(&mut all, None, nine_in_sao_paulo(Vec::new()), FRIDAY).expect("saved");
    assert_eq!(all[0].next_at, Some(FRIDAY + 12 * HOUR));
    paused(&mut all, &id, true, FRIDAY).expect("paused");
    assert!(all[0].paused);
    paused(&mut all, &id, false, FRIDAY + 3 * 24 * HOUR).expect("resumed");
    assert!(!all[0].paused);
    assert_eq!(all[0].next_at, Some(FRIDAY + 3 * 24 * HOUR + 12 * HOUR));
    let mut wrong = nine_in_sao_paulo(Vec::new());
    wrong.days = vec![7];
    assert!(saved(&mut all, None, wrong, FRIDAY).is_err());
}

#[test]
fn schedules_list_every_reminder_reached_and_every_routine_with_its_state() {
    let (_dir, store, here, projects) = store();
    let folder = Path::new(&here.root_path);
    let mut all = routines(&store, folder);
    // Set Friday afternoon, after that day's run: next on Monday.
    let afternoon = FRIDAY + 13 * HOUR;
    let id = saved(
        &mut all,
        None,
        nine_in_sao_paulo(vec![1, 2, 3, 4, 5]),
        afternoon,
    )
    .expect("saved");
    paused(&mut all, &id, true, afternoon).expect("paused");
    keep_routines(&store, folder, &all).expect("kept");

    let listed = listed(&store, &here, &projects, FRIDAY).schedules;
    let titles: Vec<&str> = listed.iter().map(|one| one.title.as_str()).collect();
    assert_eq!(
        titles,
        ["Call orch", "Call linked", "Morning summary"],
        "soonest first"
    );
    let reminder = &listed[1];
    assert_eq!(reminder.kind, ScheduleKind::Reminder);
    assert_eq!(reminder.state, ScheduleState::Waiting);
    assert_eq!(reminder.project.as_deref(), Some(projects[1].name.as_str()));
    assert_eq!(reminder.card_number, Some(1));
    let routine = &listed[2];
    assert_eq!(routine.kind, ScheduleKind::Routine);
    assert_eq!(routine.state, ScheduleState::Paused);
    assert_eq!(routine.routine.as_ref().map(|one| one.days.len()), Some(5));
    assert_eq!(
        routine.at,
        Some((FRIDAY + 3 * 24 * HOUR + 12 * HOUR) as f64)
    );

    store
        .handle_reminder(&reminder.id, FRIDAY + 21 * HOUR)
        .expect("done");
    let after = super::listed(&store, &here, &projects, FRIDAY + 22 * HOUR).schedules;
    assert_eq!(after[1].state, ScheduleState::Done);
}

#[test]
fn a_proposed_schedule_appears_only_once_the_person_adds_it() {
    let (_dir, store, here, projects) = store();
    let folder = Path::new(&here.root_path);
    let asked = json!({ "why": "You asked for CI news each morning", "routine": "ci", "time": "08:30", "days": [1, 2, 3, 4, 5] });
    let proposal = proposed(&store, folder, &here, &asked, FRIDAY).expect("proposed");
    keep_proposals(&store, folder, std::slice::from_ref(&proposal)).expect("kept");

    let waiting = listed(&store, &here, &projects, FRIDAY);
    assert_eq!(waiting.proposed.len(), 1);
    assert!(waiting
        .schedules
        .iter()
        .all(|one| one.kind == ScheduleKind::Reminder));

    let noon = FRIDAY + 12 * HOUR;
    decided(
        &store,
        &here,
        &projects,
        &proposal.id,
        true,
        ("America/Sao_Paulo", -180),
        noon,
    )
    .expect("added");
    let added = listed(&store, &here, &projects, FRIDAY);
    assert!(added.proposed.is_empty());
    let routine = added
        .schedules
        .iter()
        .find(|one| one.kind == ScheduleKind::Routine)
        .expect("the routine");
    assert_eq!(routine.title, "CI failures");
    assert_eq!(
        routine.routine.as_ref().map(|one| one.offset_minutes),
        Some(-180)
    );
    // 08:30 in São Paulo on a weekday, after Friday's: Monday, 11:30 UTC.
    assert_eq!(
        routine.at,
        Some((FRIDAY + 3 * 24 * HOUR + 11 * HOUR + 30 * 60) as f64)
    );
}

#[test]
fn a_dropped_proposal_schedules_nothing() {
    let (_dir, store, here, projects) = store();
    let folder = Path::new(&here.root_path);
    let asked = json!({ "why": "PRs pile up", "routine": "prs", "time": "17:00" });
    let proposal = proposed(&store, folder, &here, &asked, FRIDAY).expect("proposed");
    keep_proposals(&store, folder, std::slice::from_ref(&proposal)).expect("kept");
    decided(
        &store,
        &here,
        &projects,
        &proposal.id,
        false,
        ("UTC", 0),
        FRIDAY,
    )
    .expect("dropped");
    let after = listed(&store, &here, &projects, FRIDAY);
    assert!(after.proposed.is_empty());
    assert!(routines(&store, folder).is_empty());
    // Without a reason, or with a time that is none, nothing is proposed.
    assert!(proposed(
        &store,
        folder,
        &here,
        &json!({ "routine": "ci", "time": "08:30" }),
        FRIDAY
    )
    .is_err());
    assert!(proposed(
        &store,
        folder,
        &here,
        &json!({ "why": "x", "routine": "ci", "time": "8h" }),
        FRIDAY
    )
    .is_err());
}
