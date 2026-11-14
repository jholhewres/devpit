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

#[test]
fn old_logs_and_runs_go_but_the_newest_run_of_each_card_and_recent_ones_stay() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let (logs, runs, reviews) = (root.join("logs"), root.join("runs"), root.join("reviews"));
    file(&logs.join("old.jsonl"), 40 * DAY);
    file(&logs.join("new.jsonl"), DAY);
    folder(&runs.join("card_a/20261001T000000Z"), 40 * DAY);
    folder(&runs.join("card_a/20261002T000000Z"), 39 * DAY);
    folder(&runs.join("card_b/20261001T000000Z"), 40 * DAY);
    folder(&reviews.join("pr-9/20261001T000000Z"), 40 * DAY);
    folder(&reviews.join("pr-9/20261113T000000Z"), DAY);

    let mut found = stale(
        &logs,
        &[runs.clone(), reviews.clone()],
        SystemTime::now() - 30 * DAY,
    );
    found.sort();
    assert_eq!(
        found,
        vec![
            logs.join("old.jsonl"),
            reviews.join("pr-9/20261001T000000Z"),
            runs.join("card_a/20261001T000000Z"),
        ]
    );
}

#[cfg(unix)]
#[test]
fn a_link_is_never_followed_out_of_devpits_folder() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("outside");
    file(&outside.path().join("precious.txt"), 40 * DAY);
    folder(&outside.path().join("20261001T000000Z"), 40 * DAY);
    let logs = dir.path().join("logs");
    let runs = dir.path().join("runs");
    std::fs::create_dir_all(&logs).expect("logs");
    std::fs::create_dir_all(runs.join("card_a")).expect("runs");
    std::os::unix::fs::symlink(outside.path().join("precious.txt"), logs.join("linked"))
        .expect("link");
    std::os::unix::fs::symlink(outside.path(), runs.join("card_b")).expect("link");
    std::os::unix::fs::symlink(
        outside.path().join("20261001T000000Z"),
        runs.join("card_a/20260901T000000Z"),
    )
    .expect("link");

    assert!(stale(&logs, &[runs], SystemTime::now()).is_empty());
    let run = delete(dir.path(), &[outside.path().join("precious.txt")], 1.0);
    assert_eq!(run.removed, 0);
    assert!(outside.path().join("precious.txt").exists());
}

#[test]
fn deleting_says_how_many_went_and_what_it_freed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let log = dir.path().join("logs/old.jsonl");
    std::fs::create_dir_all(dir.path().join("logs")).expect("logs");
    std::fs::write(&log, vec![b'x'; 1000]).expect("write");
    let run_dir = dir.path().join("runs/card_a/20261001T000000Z");
    std::fs::create_dir_all(&run_dir).expect("run");
    std::fs::write(run_dir.join("unit.log"), vec![b'y'; 500]).expect("write");

    let run = delete(dir.path(), &[log.clone(), run_dir.clone()], 7.0);
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
