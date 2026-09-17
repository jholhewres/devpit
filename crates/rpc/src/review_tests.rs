//! A review, read back out of what an agent said.

use super::*;

fn answer(findings: &str) -> String {
    format!(r#"{{"findings": {findings}}}"#)
}

const ONE: &str = r#"[{"file": "src/main.rs", "line": 12, "severity": "blocking", "why": "the lock is never released"}]"#;

#[test]
fn a_review_reads_back_as_findings_rather_than_prose() {
    let review = reviewed(&answer(ONE), Some("2f0bee9".to_owned()), None).expect("a review");

    assert_eq!(review.findings.len(), 1);
    assert_eq!(review.findings[0].file, "src/main.rs");
    assert_eq!(review.findings[0].line, Some(12));
    assert_eq!(review.findings[0].severity, Severity::Blocking);
    assert_eq!(review.at_revision.as_deref(), Some("2f0bee9"));
}

/// A finding about the file rather than a place in it is a finding.
#[test]
fn a_finding_may_be_about_a_file_and_not_a_line() {
    let said = answer(
        r#"[{"file": "src/main.rs", "line": null, "severity": "noted", "why": "no tests"}]"#,
    );
    assert_eq!(
        reviewed(&said, None, None).expect("a review").findings[0].line,
        None
    );
}

/// The refusal is the feature. A half-read review that produced three findings
/// out of five would be one somebody acts on believing it was all of them.
///
/// Sabotage: return `Some` on a parse failure and an agent that answered prose
/// produces an empty review that reads like "nothing found".
#[test]
fn an_answer_that_does_not_parse_is_no_review_at_all() {
    assert_eq!(reviewed("looks good to me", None, None), None);
    assert_eq!(reviewed("{}", None, None), None);
    assert_eq!(reviewed(r#"{"findings": "none"}"#, None, None), None);
}

/// A finding with no reason is a claim, and a claim nobody can evaluate is not
/// something to put a line number on.
#[test]
fn a_finding_with_no_reason_refuses_the_whole_review() {
    let said =
        answer(r#"[{"file": "src/main.rs", "line": 12, "severity": "blocking", "why": "  "}]"#);
    assert_eq!(reviewed(&said, None, None), None);
}

#[test]
fn a_finding_with_no_file_refuses_the_whole_review() {
    let said = answer(r#"[{"file": "", "line": 12, "severity": "noted", "why": "something"}]"#);
    assert_eq!(reviewed(&said, None, None), None);
}

/// Line zero is an editor's impossible line: something upstream counted from
/// zero, and every other line in the review is out by one.
///
/// Sabotage: accept it and a whole review points one line above every problem.
#[test]
fn a_line_of_zero_means_the_whole_review_is_off_by_one() {
    let said = answer(r#"[{"file": "src/main.rs", "line": 0, "severity": "noted", "why": "x"}]"#);
    assert_eq!(reviewed(&said, None, None), None);
}

/// A review that found nothing is a review, and is still not an approval.
#[test]
fn a_review_that_found_nothing_is_a_review() {
    let review = reviewed(&answer("[]"), None, None).expect("a review");
    assert_eq!(review.findings, vec![]);
    assert_eq!(blocking(&review), 0);
}

fn made_at(revision: Option<&str>) -> Review {
    Review {
        findings: vec![],
        at_revision: revision.map(str::to_owned),
        rubric: None,
    }
}

#[test]
fn a_review_of_the_code_that_is_checked_out_is_current() {
    assert_eq!(
        standing(&made_at(Some("2f0bee9")), Some("2f0bee9")),
        Standing::Current
    );
}

/// No attempt to follow a line through a diff: that is a guess dressed as a
/// fact, and a review pointing confidently at the wrong line is worse than one
/// that says it is old.
///
/// Sabotage: answer `Current` whenever a revision was recorded and every
/// finding claims to be about code it never saw.
#[test]
fn a_review_of_another_revision_is_outdated_and_not_re_pointed() {
    assert_eq!(
        standing(&made_at(Some("2f0bee9")), Some("ed16baf")),
        Standing::Outdated
    );
}

#[test]
fn a_review_nobody_anchored_says_so_rather_than_claiming_either() {
    assert_eq!(
        standing(&made_at(None), Some("2f0bee9")),
        Standing::Unanchored
    );
    assert_eq!(
        standing(&made_at(Some("2f0bee9")), None),
        Standing::Unanchored
    );
    assert_eq!(standing(&made_at(None), None), Standing::Unanchored);
}

/// The rubric is kept because a review is only as good as what it was asked to
/// look for, and one edited afterwards would leave every past review looking
/// like it answered the new one.
#[test]
fn the_rubric_is_kept_as_it_stood() {
    let review = reviewed(
        &answer(ONE),
        None,
        Some("look for unreleased locks".to_owned()),
    )
    .expect("a review");
    assert_eq!(review.rubric.as_deref(), Some("look for unreleased locks"));
}

#[test]
fn blocking_counts_only_what_would_break_something() {
    let said = answer(
        r#"[{"file": "a", "line": 1, "severity": "blocking", "why": "x"},
            {"file": "b", "line": 2, "severity": "worth", "why": "y"},
            {"file": "c", "line": 3, "severity": "noted", "why": "z"}]"#,
    );
    assert_eq!(blocking(&reviewed(&said, None, None).expect("a review")), 1);
}
