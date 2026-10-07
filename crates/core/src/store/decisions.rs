//! The decision log: every question Decisions was asked, what it answered,
//! what that cost, and what was done with it.
//!
//! Kept so a gate running in shadow can be judged before it is trusted, and so
//! the daily cap is summed from what was actually spent.

use crate::store::{Store, StoreError};

/// How long a decision is kept.
pub const DECISIONS_KEPT_DAYS: i64 = 90;

/// One decision, as it is written.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionWrite<'a> {
    pub project_id: Option<&'a str>,
    pub card_id: Option<&'a str>,
    pub gate: &'a str,
    /// `shadow` or `enforce`.
    pub mode: &'a str,
    pub rubric: Option<&'a str>,
    pub rubric_version: Option<&'a str>,
    /// JSON, as sent.
    pub questions: &'a str,
    pub answers: Option<&'a str>,
    pub thresholds: Option<&'a str>,
    /// `pass`, `fail`, `grey`, `skipped` or `answered`.
    pub outcome: &'a str,
    pub cost_usd: f64,
    pub latency_ms: i64,
    /// A hash and a length, never the state itself.
    pub state_digest: &'a str,
}

/// One decision, as it is read back.
#[derive(Debug, Clone, PartialEq)]
pub struct DecisionRow {
    pub id: String,
    pub at: i64,
    pub project_id: Option<String>,
    pub card_id: Option<String>,
    pub gate: String,
    pub mode: String,
    pub rubric: Option<String>,
    pub questions: String,
    pub answers: Option<String>,
    pub outcome: String,
    pub cost_usd: f64,
    pub latency_ms: i64,
    pub state_digest: String,
}

impl Store {
    /// Writes a decision and prunes the ones past [`DECISIONS_KEPT_DAYS`]:
    /// on write, so a log nobody reads still stays its size.
    pub fn log_decision(&self, one: &DecisionWrite<'_>, now: i64) -> Result<String, StoreError> {
        let id = format!("dcn_{}", ulid::Ulid::generate());
        let tx = self.writing()?;
        tx.execute(
            "INSERT INTO decision (id, at, project_id, card_id, gate, mode, rubric, rubric_version, \
             questions, answers, thresholds, outcome, cost_usd, latency_ms, state_digest) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
            rusqlite::params![
                id,
                now,
                one.project_id,
                one.card_id,
                one.gate,
                one.mode,
                one.rubric,
                one.rubric_version,
                one.questions,
                one.answers,
                one.thresholds,
                one.outcome,
                one.cost_usd,
                one.latency_ms,
                one.state_digest,
            ],
        )?;
        tx.execute(
            "DELETE FROM decision WHERE at < ?1",
            [now - DECISIONS_KEPT_DAYS * 86_400],
        )?;
        tx.commit()?;
        Ok(id)
    }

    /// What decisions since `since` cost, and how many were answered.
    pub fn decision_spend_since(&self, since: i64) -> Result<(f64, u32), StoreError> {
        Ok(self.conn.query_row(
            "SELECT COALESCE(SUM(cost_usd), 0.0), \
             COUNT(*) FILTER (WHERE outcome != 'skipped') FROM decision WHERE at >= ?1",
            [since],
            |row| Ok((row.get(0)?, row.get::<_, i64>(1)? as u32)),
        )?)
    }

    /// The latest decisions, newest first.
    pub fn decisions_latest(&self, most: u32) -> Result<Vec<DecisionRow>, StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT id, at, project_id, card_id, gate, mode, rubric, questions, answers, outcome, \
             cost_usd, latency_ms, state_digest FROM decision ORDER BY at DESC, id DESC LIMIT ?1",
        )?;
        let rows = statement.query_map([most], |row| {
            Ok(DecisionRow {
                id: row.get(0)?,
                at: row.get(1)?,
                project_id: row.get(2)?,
                card_id: row.get(3)?,
                gate: row.get(4)?,
                mode: row.get(5)?,
                rubric: row.get(6)?,
                questions: row.get(7)?,
                answers: row.get(8)?,
                outcome: row.get(9)?,
                cost_usd: row.get(10)?,
                latency_ms: row.get(11)?,
                state_digest: row.get(12)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

#[cfg(test)]
#[path = "decisions_tests.rs"]
mod tests;
