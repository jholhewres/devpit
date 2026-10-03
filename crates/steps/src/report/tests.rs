//! Each format against recorded reports (paths moved under `/work/app`).

use std::path::Path;

use super::*;

const CWD: &str = "/work/app";

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!("../../fixtures/reports/", $name))
    };
}

fn file(text: &str) -> Tested {
    from_file(text, Path::new(CWD)).expect("a report it recognises")
}

fn printed(text: &str) -> Option<Tested> {
    read(None, Some(text), Path::new(CWD))
}

#[test]
fn vitest_that_passed_counts_every_test() {
    let tested = file(fixture!("vitest-passed.json"));
    assert_eq!((tested.passed, tested.failed), (2, 0));
    assert_eq!(tested.read_from, ["vitest"]);
}

#[test]
fn vitest_that_failed_names_the_test_its_file_and_why() {
    let tested = file(fixture!("vitest-failed.json"));
    assert_eq!((tested.passed, tested.failed, tested.skipped), (1, 1, 1));
    let failure = &tested.failures[0];
    assert_eq!(failure.name, "the answer is wrong");
    assert_eq!(failure.file.as_deref(), Some("src/fail.test.ts"));
    assert_eq!(
        failure.message.as_deref(),
        Some("AssertionError: expected 41 to be 42 // Object.is equality")
    );
}

#[test]
fn a_vitest_file_that_never_ran_is_a_failure() {
    let tested = file(fixture!("vitest-broken-file.json"));
    assert_eq!((tested.passed, tested.failed), (2, 1));
    assert_eq!(
        tested.failures[0].file.as_deref(),
        Some("src/broken.test.ts")
    );
    assert!(tested.failures[0]
        .message
        .as_deref()
        .is_some_and(|said| said.starts_with("Cannot find module")));
}

#[test]
fn vitest_that_matched_nothing_reports_nothing_passed() {
    let tested = file(fixture!("vitest-none.json"));
    assert_eq!((tested.passed, tested.failed), (0, 0));
}

#[test]
fn cargo_test_that_passed_counts_every_binary() {
    let tested = printed(fixture!("cargo-passed.txt")).expect("a result line");
    assert_eq!((tested.passed, tested.failed, tested.skipped), (2, 0, 1));
    assert_eq!(tested.read_from, ["cargo test"]);
}

#[test]
fn cargo_test_that_failed_says_where_it_panicked() {
    let tested = printed(fixture!("cargo-failed.txt")).expect("a result line");
    assert_eq!((tested.passed, tested.failed, tested.skipped), (1, 1, 1));
    let failure = &tested.failures[0];
    assert_eq!(failure.name, "tests::the_answer_is_wrong");
    assert_eq!(failure.file.as_deref(), Some("src/lib.rs"));
    assert_eq!(
        failure.message.as_deref(),
        Some("assertion `left == right` failed: off by one")
    );
}

#[test]
fn cargo_test_that_matched_nothing_reports_nothing_passed() {
    let tested = printed(fixture!("cargo-none.txt")).expect("a result line");
    assert_eq!((tested.passed, tested.failed), (0, 0));
}

#[test]
fn libtest_json_reads_like_its_text() {
    let tested = printed(fixture!("libtest-failed.jsonl")).expect("a suite line");
    assert_eq!((tested.passed, tested.failed, tested.skipped), (1, 1, 1));
    assert_eq!(tested.failures[0].name, "tests::the_answer_is_wrong");
    assert_eq!(tested.failures[0].file.as_deref(), Some("src/lib.rs"));
}

#[test]
fn pytest_junit_counts_failures_errors_and_skips() {
    let tested = file(fixture!("pytest-failed.xml"));
    assert_eq!((tested.passed, tested.failed, tested.skipped), (1, 2, 1));
    assert_eq!(tested.read_from, ["JUnit"]);
    assert_eq!(tested.failures[0].name, "test_the_answer_is_wrong");
    assert_eq!(
        tested.failures[0].message.as_deref(),
        Some("AssertionError: off by one")
    );
}

#[test]
fn pytest_junit_that_passed_and_that_matched_nothing() {
    let passed = file(fixture!("pytest-passed.xml"));
    assert_eq!((passed.passed, passed.failed), (1, 0));
    let none = file(fixture!("pytest-none.xml"));
    assert_eq!((none.passed, none.failed), (0, 0));
}

#[test]
fn node_junit_reads_the_same_way() {
    let failed = file(fixture!("node-failed.xml"));
    assert_eq!((failed.passed, failed.failed), (1, 1));
    assert_eq!(failed.failures[0].message.as_deref(), Some("41 == 42"));
    let passed = file(fixture!("node-passed.xml"));
    assert_eq!((passed.passed, passed.failed), (1, 0));
}

#[test]
fn an_output_that_was_cut_is_not_read() {
    assert_eq!(read(None, None, Path::new(CWD)), None);
}

#[test]
fn output_with_no_result_line_reports_nothing() {
    assert_eq!(printed("error[E0425]: cannot find value `x`\n"), None);
}

#[test]
fn every_report_in_the_folder_adds_up_with_what_was_printed() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("web.json"), fixture!("vitest-failed.json")).unwrap();
    std::fs::write(dir.path().join("py.xml"), fixture!("pytest-passed.xml")).unwrap();
    std::fs::write(dir.path().join("notes.txt"), "not a report").unwrap();

    let tested = read(
        Some(dir.path()),
        Some(fixture!("cargo-passed.txt")),
        Path::new(CWD),
    )
    .expect("three reports");
    assert_eq!((tested.passed, tested.failed), (1 + 1 + 2, 1));
    assert_eq!(tested.read_from, ["JUnit", "vitest", "cargo test"]);
}

#[test]
fn a_report_past_the_ceiling_is_not_read() {
    let dir = tempfile::tempdir().expect("tempdir");
    let big = std::fs::File::create(dir.path().join("big.json")).unwrap();
    big.set_len(MOST_REPORT + 1).unwrap();
    assert_eq!(read(Some(dir.path()), None, Path::new(CWD)), None);
}

#[cfg(unix)]
#[test]
fn a_link_in_the_folder_is_not_followed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outside = tempfile::tempdir().expect("tempdir");
    let target = outside.path().join("report.json");
    std::fs::write(&target, fixture!("vitest-passed.json")).unwrap();
    std::os::unix::fs::symlink(&target, dir.path().join("report.json")).unwrap();
    assert_eq!(read(Some(dir.path()), None, Path::new(CWD)), None);
}

#[test]
fn a_long_message_is_cut_and_loses_its_colour() {
    let said = short(&format!("\u{1b}[31m{}\u{1b}[39m\nmore", "x".repeat(400))).unwrap();
    assert!(said.starts_with("xxx") && said.ends_with('…'));
    assert_eq!(said.chars().count(), LONGEST_MESSAGE + 1);
}
