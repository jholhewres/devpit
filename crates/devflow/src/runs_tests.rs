use std::time::Duration;

use super::*;

fn at(seconds: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(seconds)
}

fn names_in(folder: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .expect("read")
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

#[test]
fn a_run_is_named_by_its_time_in_utc() {
    assert_eq!(stamp(at(1_791_729_005)), "20261011T143005Z");
    assert_eq!(stamp(at(0)), "19700101T000000Z");
    assert_eq!(stamp(at(951_782_400)), "20000229T000000Z");
    assert_eq!(stamp(at(1_798_761_599)), "20261231T235959Z");
}

#[test]
fn a_second_run_in_the_same_second_gets_a_folder_of_its_own() {
    let root = tempfile::tempdir().expect("root");
    let (first, path) = fresh(root.path(), "card_1", at(1_791_729_005)).expect("first");
    assert_eq!(first, ".devpit/devflow/runs/card_1/20261011T143005Z");
    assert!(path.is_dir());
    let (second, _) = fresh(root.path(), "card_1", at(1_791_729_005)).expect("second");
    assert_eq!(second, ".devpit/devflow/runs/card_1/20261011T143005Z-1");
    assert!(fresh(root.path(), "../card", at(0)).is_err());
}

#[cfg(unix)]
#[test]
fn a_runs_folder_that_leads_outside_the_checkout_is_refused() {
    let root = tempfile::tempdir().expect("root");
    let elsewhere = tempfile::tempdir().expect("elsewhere");
    std::fs::create_dir_all(root.path().join(".devpit/devflow")).expect("folder");
    std::os::unix::fs::symlink(elsewhere.path(), root.path().join(RUNS)).expect("link");
    assert!(fresh(root.path(), "card_1", at(0)).is_err());
    assert!(names_in(elsewhere.path()).is_empty());
}

#[test]
fn only_the_newest_runs_are_kept_and_nothing_else_is_touched() {
    let folder = tempfile::tempdir().expect("folder");
    let runs = [
        "20261011T143005Z",
        "20261011T143005Z-1",
        "20261011T143005Z-10",
        "20261011T143005Z-2",
        "20261012T090000Z",
        "20250101T000000Z",
    ];
    for name in runs.iter().chain(&["local"]) {
        std::fs::create_dir(folder.path().join(name)).expect("run");
    }
    std::fs::write(folder.path().join("20200101T000000Z.log"), "x").expect("file");

    let mut gone = prune(folder.path(), 3);
    gone.sort();
    assert_eq!(
        gone,
        ["20250101T000000Z", "20261011T143005Z", "20261011T143005Z-1"]
    );
    assert_eq!(
        names_in(folder.path()),
        [
            "20200101T000000Z.log",
            "20261011T143005Z-10",
            "20261011T143005Z-2",
            "20261012T090000Z",
            "local"
        ]
    );
}

#[test]
fn a_checks_pictures_and_videos_join_the_run_when_its_evidence_keeps_them() {
    let run = tempfile::tempdir().expect("run");
    let results = run.path().join("e2e/test-results/rename-chromium");
    std::fs::create_dir_all(&results).expect("results");
    std::fs::write(results.join("test-finished-1.png"), "png").expect("shot");
    std::fs::write(results.join("video.webm"), "webm").expect("video");
    std::fs::write(results.join("trace.zip"), "zip").expect("trace");

    let media = gather(run.path(), "e2e", &[Evidence::Screenshots]);
    assert_eq!(
        media.screenshots,
        ["screenshots/e2e-test-results-rename-chromium-test-finished-1.png"]
    );
    assert!(media.video.is_empty(), "video is not kept: {media:?}");
    assert!(run.path().join(&media.screenshots[0]).is_file());

    let video = gather(run.path(), "e2e", &[Evidence::Video]);
    assert_eq!(
        video.video,
        ["video/e2e-test-results-rename-chromium-video.webm"]
    );
    assert!(results.join("trace.zip").is_file());
}

#[test]
fn attaching_copies_the_runs_files_into_the_artifacts_under_the_card() {
    let run = tempfile::tempdir().expect("run");
    let artifacts = tempfile::tempdir().expect("artifacts");
    std::fs::create_dir_all(run.path().join("e2e")).expect("e2e");
    std::fs::write(run.path().join("e2e/report.xml"), "<testsuites/>").expect("report");
    std::fs::write(run.path().join("evidence.json"), "{}").expect("evidence");

    let under = format!("{}/20261011T143005Z", attached_under("card_1"));
    let files = [
        "e2e/report.xml".to_owned(),
        "evidence.json".to_owned(),
        "gone.png".to_owned(),
    ];
    let attached = attach(run.path(), &files, artifacts.path(), &under);
    assert_eq!(
        attached.names,
        [
            "devflow/card_1/20261011T143005Z/e2e/report.xml",
            "devflow/card_1/20261011T143005Z/evidence.json"
        ]
    );
    assert_eq!(attached.left_out, ["gone.png"]);
    assert_eq!(
        std::fs::read_to_string(artifacts.path().join(&attached.names[0])).expect("copied"),
        "<testsuites/>"
    );
}

#[test]
fn the_setups_gitignore_keeps_every_run_out_of_the_repository() {
    let root = tempfile::tempdir().expect("root");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(root.path())
            .output()
            .expect("git runs")
    };
    assert!(git(&["init", "-q"]).status.success());
    let files = crate::setup::planned("version: 1\n", &[], None);
    crate::setup::write(root.path(), &files).expect("setup");
    let (_, run) = fresh(root.path(), "card_1", at(1_791_729_005)).expect("run");
    std::fs::write(run.join("evidence.json"), "{}").expect("evidence");

    let ignored = git(&[
        "check-ignore",
        "-q",
        ".devpit/devflow/runs/card_1/20261011T143005Z/evidence.json",
    ]);
    assert!(ignored.status.success(), "runs/ is not ignored");
    let kept = git(&["check-ignore", "-q", ".devpit/devflow/devflow.yaml"]);
    assert!(!kept.status.success(), "devflow.yaml is ignored");
}

#[test]
fn a_file_of_the_runs_goes_inside_its_run_or_beside_the_runs_and_nowhere_else() {
    let root = tempfile::tempdir().expect("root");
    let (inside, path) =
        file_in(root.path(), "card_1", Some("20261011T143005Z"), "gate.json").expect("in a run");
    assert_eq!(
        inside,
        ".devpit/devflow/runs/card_1/20261011T143005Z/gate.json"
    );
    assert!(path.parent().expect("its run").is_dir());
    let (beside, _) = file_in(root.path(), "card_1", None, "gate.json").expect("beside");
    assert_eq!(beside, ".devpit/devflow/runs/card_1/gate.json");
    assert!(file_in(root.path(), "card_1", Some("../x"), "gate.json").is_err());
    assert!(file_in(root.path(), "../card", None, "gate.json").is_err());
}

#[test]
fn a_runs_name_reads_back_as_the_second_it_was_made() {
    for seconds in [0, 951_782_400, 1_791_729_005, 4_102_444_799] {
        assert_eq!(seconds_of(&stamp(at(seconds))), Some(seconds), "{seconds}");
    }
    assert_eq!(
        seconds_of("20261011T143005Z-2"),
        seconds_of("20261011T143005Z")
    );
    assert_eq!(seconds_of("local"), None);
}

#[test]
fn the_newest_run_holding_a_file_is_found_and_a_newer_one_without_it_skipped() {
    let folder = tempfile::tempdir().expect("folder");
    for (name, file) in [
        ("20261011T120000Z", "evidence.json"),
        ("20261011T120000Z-1", "evidence.json"),
        ("20261011T130000Z", "findings.json"),
        ("not-a-run", "evidence.json"),
    ] {
        std::fs::create_dir_all(folder.path().join(name)).expect("run");
        std::fs::write(folder.path().join(name).join(file), "{}").expect("file");
    }
    assert_eq!(
        newest_with(folder.path(), "evidence.json").as_deref(),
        Some("20261011T120000Z-1")
    );
    assert_eq!(
        newest_with(folder.path(), "findings.json").as_deref(),
        Some("20261011T130000Z")
    );
    assert_eq!(newest_with(folder.path(), "gate.json"), None);
}
