//! The project's runs, as the runs view pages through them.
//!
//! Apart from `board.rs`, which is at its size ceiling and is the shape of a
//! board; this is history across every card on it.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::board::{Run, RunState};

/// Where the last page ended: the next one starts after this run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunCursor {
    pub started_at: f64,
    pub id: String,
}

/// What to list. Every filter is optional, and they narrow together.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunsQuery {
    pub project_id: String,
    /// A lane is filtered by the step it runs.
    pub step_id: Option<String>,
    pub state: Option<RunState>,
    /// Unix seconds, inclusive.
    pub since: Option<f64>,
    /// Unix seconds, exclusive.
    pub until: Option<f64>,
    pub after: Option<RunCursor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRun {
    pub run: Run,
    pub card_id: String,
    pub card_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RunsPage {
    pub runs: Vec<ProjectRun>,
    /// Absent on the last page.
    pub next: Option<RunCursor>,
}
