//! A review an agent gave, as findings rather than prose.
//!
//! Prose is what a review looked like before this: a paragraph on a card,
//! which nobody can act on twice. A finding is a file, a line, how bad, and
//! why — four things a person can open, sort and argue with.
//!
//! **Anchored to the revision it was born in.** A line number is only a
//! location while the file holds still. Edit above it and the same number
//! points at different code, and a review that silently followed would be
//! telling somebody about a problem at a place that no longer has one. So a
//! finding carries the revision it was made against, and a later look says
//! `Outdated` rather than re-pointing the line.
//!
//! **An invalid answer is not an approval.** Everything here parses or it
//! does not; a review that fails to read produces no findings and no verdict,
//! and the rest of the app treats that as silence. The same rule the verdict
//! field already follows: silence is not approval.

use serde::{Deserialize, Serialize};
use specta::Type;

/// The shape of the payload a review writes into a run's evidence.
///
/// On the row beside the payload, so a reader that does not know this number
/// says so instead of reading the bytes as though they were the shape it
/// knows. Bumped when a field's meaning changes, never when one is added.
pub const REVIEW_EVIDENCE: i64 = 1;

/// How much a finding is claimed to matter.
///
/// A closed set, and small. Seven levels is a scale nobody agrees on twice,
/// and a review's job is to be acted on rather than scored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// Would break something if it shipped.
    Blocking,
    /// Worth fixing, not worth stopping for.
    Worth,
    /// Said for the record.
    Noted,
}

/// One thing a review found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    /// Relative to the checkout the review ran in.
    pub file: String,
    /// 1-based, as an editor counts. `None` for a finding about the file
    /// rather than a place in it.
    pub line: Option<u32>,
    pub severity: Severity,
    /// Why this is a finding. Empty is refused: a finding nobody can evaluate
    /// is a claim, not a finding.
    pub why: String,
}

/// A review, and what it was a review of.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub findings: Vec<Finding>,
    /// The commit the reviewed code was at. A finding's line means this
    /// revision and no other.
    pub at_revision: Option<String>,
    /// The rubric the agent was given, as it stood then.
    ///
    /// Kept because a review is only as good as what it was asked to look
    /// for, and a rubric edited afterwards would leave every past review
    /// looking like it answered the new one.
    pub rubric: Option<String>,
}

/// Whether a finding is still about the code in front of you.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Standing {
    /// Made against the revision that is checked out.
    Current,
    /// Made against another. The line number is not re-pointed, and the
    /// screen says which revision it meant.
    Outdated,
    /// Nothing recorded which revision it was made against.
    Unanchored,
}

/// Where a review stands against the code now.
///
/// Deliberately not clever. There is no attempt to follow a line through a
/// diff: that is a guess dressed as a fact, and a review pointing confidently
/// at the wrong line is worse than one that says it is old.
pub fn standing(review: &Review, revision: Option<&str>) -> Standing {
    match (&review.at_revision, revision) {
        (Some(made), Some(now)) if made == now => Standing::Current,
        (Some(_), Some(_)) => Standing::Outdated,
        _ => Standing::Unanchored,
    }
}

/// Reads a review out of an agent's answer, or refuses it.
///
/// `None` for anything that does not parse into findings that are all
/// complete. That refusal is the feature: a half-read review that produced
/// three findings out of five would be a review somebody acts on believing it
/// was all of them.
pub fn reviewed(
    answer: &str,
    at_revision: Option<String>,
    rubric: Option<String>,
) -> Option<Review> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Answered {
        findings: Vec<Finding>,
    }

    let answered: Answered = serde_json::from_str(answer).ok()?;
    // A finding with no reason is a claim. A line of zero is an editor's
    // impossible line, which means something upstream counted from zero and
    // every other line in the review is out by one.
    if answered.findings.iter().any(|found| {
        found.why.trim().is_empty() || found.file.trim().is_empty() || found.line == Some(0)
    }) {
        return None;
    }
    Some(Review {
        findings: answered.findings,
        at_revision,
        rubric,
    })
}

/// Whether a review approves anything.
///
/// It does not, ever. A review reports; a verdict field is what approves, and
/// that is checked elsewhere. This exists so nothing is tempted to read "no
/// blocking findings" as a pass — a review that found nothing may have looked
/// at nothing.
pub fn blocking(review: &Review) -> usize {
    review
        .findings
        .iter()
        .filter(|found| found.severity == Severity::Blocking)
        .count()
}

/// A review as the screen reads it: what it found, and whether it still
/// points where it says.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub findings: Vec<Finding>,
    pub standing: Standing,
    /// The revision the review was made against.
    pub at_revision: Option<String>,
    /// The revision the code is at now. Both are shown when they differ: a
    /// person reading an outdated review needs to know which two.
    pub now: Option<String>,
    pub rubric: Option<String>,
}

impl Found {
    /// A run that left no review. Not an empty review — a review that found
    /// nothing looked at something, and this did not.
    pub fn none() -> Self {
        Self {
            findings: vec![],
            standing: Standing::Unanchored,
            at_revision: None,
            now: None,
            rubric: None,
        }
    }
}

#[cfg(test)]
#[path = "review_tests.rs"]
mod tests;
