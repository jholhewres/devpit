use std::path::Path;
use std::time::{Duration, SystemTime};

use super::*;

const DAY: Duration = Duration::from_secs(24 * 60 * 60);

fn aged(path: &Path, ago: Duration) {
    std::fs::File::open(path)
        .expect("open")
        .set_modified(SystemTime::now() - ago)
        .expect("set the time");
}

fn folder(path: &Path, ago: Duration) {
    std::fs::create_dir_all(path).expect("mkdir");
    std::fs::write(path.join("report.json"), "{}").expect("write");
    aged(path, ago);
}

fn file(path: &Path, ago: Duration) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, "a line\n").expect("write");
    aged(path, ago);
}

#[test]
fn power_saver_runs_a_probe_four_times_less_often_only_while_saving() {
    let every = Duration::from_secs(2);
    assert_eq!(slower(every, false), every);
    assert_eq!(slower(every, true), Duration::from_secs(8));
}

#[test]
fn only_a_battery_that_is_discharging_counts_as_on_battery() {
    let dir = tempfile::tempdir().expect("tempdir");
    let supply = |name: &str, kind: &str, status: &str| {
        let at = dir.path().join(name);
        std::fs::create_dir_all(&at).expect("mkdir");
        std::fs::write(at.join("type"), format!("{kind}\n")).expect("type");
        std::fs::write(at.join("status"), format!("{status}\n")).expect("status");
    };
    supply("AC", "Mains", "");
    assert!(!on_battery_in(dir.path()));
    supply("BAT0", "Battery", "Charging");
    assert!(!on_battery_in(dir.path()));
    supply("BAT0", "Battery", "Discharging");
    assert!(on_battery_in(dir.path()));
    assert!(!on_battery_in(&dir.path().join("nowhere")));
}

/// A run's folder name, `ago` before now, as Devflow names it.
fn run_name(ago: Duration) -> String {
    devflow_engine::runs::stamp(SystemTime::now() - ago)
}

#[test]
fn old_logs_and_runs_go_but_the_newest_run_of_each_card_and_recent_ones_stay() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let (logs, runs, reviews) = (root.join("logs"), root.join("runs"), root.join("reviews"));
    let (old, older, recent) = (run_name(39 * DAY), run_name(40 * DAY), run_name(DAY));
    file(&logs.join("old.jsonl"), 40 * DAY);
    file(&logs.join("new.jsonl"), DAY);
    folder(&runs.join("card_a").join(&older), DAY);
    folder(&runs.join("card_a").join(&old), DAY);
    folder(&runs.join("card_b").join(&older), DAY);
    folder(&reviews.join("pr-9").join(&older), DAY);
    folder(&reviews.join("pr-9").join(&recent), DAY);

    let mut found = stale(
        &logs,
        &[runs.clone(), reviews.clone()],
        SystemTime::now() - 30 * DAY,
    );
    found.sort();
    let mut expected = vec![
        logs.join("old.jsonl"),
        reviews.join("pr-9").join(&older),
        runs.join("card_a").join(&older),
    ];
    expected.sort();
    assert_eq!(found, expected);
}

#[test]
fn a_folder_that_is_not_a_run_is_never_taken_for_the_newest_nor_deleted() {
    let dir = tempfile::tempdir().expect("tempdir");
    let runs = dir.path().join("runs");
    let (older, old) = (run_name(41 * DAY), run_name(40 * DAY));
    folder(&runs.join("card_a").join(&older), 40 * DAY);
    folder(&runs.join("card_a").join(&old), 40 * DAY);
    folder(&runs.join("card_a/proofs"), 40 * DAY);
    folder(&runs.join("card_a/local"), 40 * DAY);

    let found = stale(
        &dir.path().join("logs"),
        std::slice::from_ref(&runs),
        SystemTime::now() - 30 * DAY,
    );
    assert_eq!(found, vec![runs.join("card_a").join(&older)]);
}

#[test]
fn the_newest_evidence_and_review_a_card_reads_stay_however_old() {
    let dir = tempfile::tempdir().expect("tempdir");
    let card = dir.path().join("runs/card_a");
    let (evidence, review, newest, oldest) = (
        run_name(50 * DAY),
        run_name(45 * DAY),
        run_name(40 * DAY),
        run_name(60 * DAY),
    );
    for (name, file) in [
        (&oldest, "evidence.json"),
        (&evidence, "evidence.json"),
        (&evidence, "gate.json"),
        (&review, "findings.json"),
        (&review, "review.json"),
        (&newest, "review-context.md"),
    ] {
        std::fs::create_dir_all(card.join(name)).expect("run");
        std::fs::write(card.join(name).join(file), "{}").expect("write");
    }

    let found = stale(
        &dir.path().join("logs"),
        &[dir.path().join("runs")],
        SystemTime::now() - 30 * DAY,
    );
    assert_eq!(found, vec![card.join(&oldest)]);
}

#[cfg(unix)]
#[test]
fn a_link_is_never_followed_out_of_the_folder_being_cleaned() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    let older = run_name(40 * DAY);
    file(&outside.path().join("precious.txt"), 40 * DAY);
    folder(&outside.path().join(&older), 40 * DAY);
    folder(&outside.path().join("card_c").join(&older), 40 * DAY);
    folder(
        &outside.path().join("card_c").join(run_name(39 * DAY)),
        40 * DAY,
    );
    let logs = dir.path().join("logs");
    let runs = dir.path().join("runs");
    std::fs::create_dir_all(&logs).expect("logs");
    std::fs::create_dir_all(runs.join("card_a")).expect("runs");
    std::os::unix::fs::symlink(outside.path().join("precious.txt"), logs.join("linked"))
        .expect("link");
    std::os::unix::fs::symlink(outside.path(), runs.join("card_b")).expect("link");
    std::os::unix::fs::symlink(
        outside.path().join(&older),
        runs.join("card_a").join(&older),
    )
    .expect("link");
    // A whole tree that is a link, even to somewhere inside devpit's home.
    let linked_tree = dir.path().join("reviews");
    std::os::unix::fs::symlink(outside.path(), &linked_tree).expect("link");

    assert!(stale(
        &logs,
        &[runs.clone(), linked_tree.clone()],
        SystemTime::now()
    )
    .is_empty());
    let run = delete(
        &[logs, runs, linked_tree],
        &[
            outside.path().join("precious.txt"),
            outside.path().join("card_c").join(&older),
        ],
        1.0,
    );
    assert_eq!(run.removed, 0);
    assert!(outside.path().join("precious.txt").exists());
    assert!(outside.path().join("card_c").join(&older).exists());
}

#[test]
fn deleting_says_how_many_went_and_what_it_freed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let logs = dir.path().join("logs");
    let runs = dir.path().join("runs");
    let log = logs.join("old.jsonl");
    std::fs::create_dir_all(&logs).expect("logs");
    std::fs::write(&log, vec![b'x'; 1000]).expect("write");
    let run_dir = runs.join("card_a").join(run_name(40 * DAY));
    std::fs::create_dir_all(&run_dir).expect("run");
    std::fs::write(run_dir.join("unit.log"), vec![b'y'; 500]).expect("write");

    let run = delete(&[logs, runs], &[log.clone(), run_dir.clone()], 7.0);
    assert_eq!(run.removed, 2);
    assert!(run.freed_bytes >= 1500.0, "{run:?}");
    assert_eq!(run.at, 7.0);
    assert!(!log.exists() && !run_dir.exists());
}

#[test]
fn defaults_stay_as_before_and_labs_flags_turn_off() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let shown = read(&store).expect("read");
    assert!(!shown.power_saver);
    assert_eq!(shown.keep_days, 0);
    assert!(shown.project_agent && shown.canvas);

    store
        .set_preference_flag(preference::LABS_PROJECT_AGENT, false)
        .expect("flag");
    store
        .set_preference_flag(preference::LABS_CANVAS, false)
        .expect("flag");
    assert!(!project_agent_on(&store));
    assert!(!canvas_on(&store));
}

#[test]
fn days_that_settings_does_not_offer_keep_everything() {
    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    store
        .set_preference(preference::MAINTENANCE_KEEP_DAYS, "30")
        .expect("days");
    assert_eq!(keep_days(&store), 30);
    store
        .set_preference(preference::MAINTENANCE_KEEP_DAYS, "3")
        .expect("days");
    assert_eq!(keep_days(&store), 0);
}
