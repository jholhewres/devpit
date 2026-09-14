//! Paging and filtering a project's runs.

use crate::store::project_runs::RunQuery;
use crate::store::Store;

struct Seeded {
    _dir: tempfile::TempDir,
    store: Store,
    project: String,
    card: String,
    tests: String,
    deploy: String,
}

fn seeded() -> Seeded {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let root = dir.path().join("project");
    std::fs::create_dir_all(&root).expect("create");
    let project = store.add_project(&root, None).expect("project");
    store.ensure_board(&project).expect("board");
    let first = store.columns(&project).expect("columns")[0].id.clone();
    let card = store
        .create_card(&project, &first, "a card", "")
        .expect("card");
    let tests = store
        .create_step(&project, "command", "tests", "{}", false)
        .expect("step");
    let deploy = store
        .create_step(&project, "command", "deploy", "{}", true)
        .expect("step");
    Seeded {
        _dir: dir,
        store,
        project,
        card,
        tests,
        deploy,
    }
}

/// A finished run started at a chosen second, so order is not left to the clock.
fn run_at(store: &Store, card: &str, step: &str, state: &str, at: i64) -> String {
    let id = store.start_run(card, step, None).expect("start");
    store
        .finish_run(&id, state, None, None, None, None)
        .expect("finish");
    store
        .conn()
        .execute(
            "UPDATE run SET started_at = ?1 WHERE id = ?2",
            rusqlite::params![at, id],
        )
        .expect("date");
    id
}

fn all(project: &str) -> RunQuery<'_> {
    RunQuery {
        project_id: project,
        limit: 50,
        ..Default::default()
    }
}

/// Pages continue from the last row rather than an offset, and runs that share
/// a second are still ordered, so nothing repeats and nothing is skipped.
#[test]
fn pages_follow_on_without_repeating_or_skipping() {
    let s = seeded();
    let mut made: Vec<(i64, String)> = [100, 101, 102, 105, 105, 105, 103]
        .into_iter()
        .map(|at| (at, run_at(&s.store, &s.card, &s.tests, "ok", at)))
        .collect();
    made.sort_by(|a, b| b.cmp(a));

    let mut seen: Vec<(i64, String)> = Vec::new();
    loop {
        let before = seen.last().map(|(at, id)| (*at, id.as_str()));
        let page = s
            .store
            .project_runs(&RunQuery {
                // Two, so the three runs sharing second 105 split across pages.
                limit: 2,
                before,
                ..all(&s.project)
            })
            .expect("page");
        if page.is_empty() {
            break;
        }
        seen.extend(page.into_iter().map(|run| (run.started_at, run.id)));
    }
    assert_eq!(seen, made);
}

#[test]
fn filters_narrow_together() {
    let s = seeded();
    let old_ok = run_at(&s.store, &s.card, &s.tests, "ok", 100);
    let failed = run_at(&s.store, &s.card, &s.tests, "failed", 200);
    let deployed = run_at(&s.store, &s.card, &s.deploy, "ok", 300);
    let ids = |query: RunQuery<'_>| -> Vec<String> {
        s.store
            .project_runs(&query)
            .expect("list")
            .into_iter()
            .map(|run| run.id)
            .collect()
    };

    assert_eq!(
        ids(RunQuery {
            step_id: Some(&s.tests),
            ..all(&s.project)
        }),
        vec![failed.clone(), old_ok.clone()]
    );
    assert_eq!(
        ids(RunQuery {
            state: Some("ok"),
            ..all(&s.project)
        }),
        vec![deployed.clone(), old_ok.clone()]
    );
    // Since is inclusive and until exclusive, so adjacent ranges never overlap.
    assert_eq!(
        ids(RunQuery {
            since: Some(200),
            until: Some(300),
            ..all(&s.project)
        }),
        vec![failed.clone()]
    );
    assert_eq!(
        ids(RunQuery {
            step_id: Some(&s.tests),
            state: Some("ok"),
            ..all(&s.project)
        }),
        vec![old_ok]
    );
}

#[test]
fn another_projects_runs_are_not_listed() {
    let s = seeded();
    run_at(&s.store, &s.card, &s.tests, "ok", 100);
    let root = s._dir.path().join("other");
    std::fs::create_dir_all(&root).expect("create");
    let other = s.store.add_project(&root, None).expect("project");
    assert!(s.store.project_runs(&all(&other)).expect("list").is_empty());
}

/// A page holds at least one run and never more than the ceiling, whatever
/// the caller asked for.
#[test]
fn a_page_size_is_kept_within_bounds() {
    let s = seeded();
    run_at(&s.store, &s.card, &s.tests, "ok", 100);
    run_at(&s.store, &s.card, &s.tests, "ok", 101);
    let listed = s
        .store
        .project_runs(&RunQuery {
            limit: 0,
            ..all(&s.project)
        })
        .expect("list");
    assert_eq!(listed.len(), 1);
}
