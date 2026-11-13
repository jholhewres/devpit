//! How an answer ended, kept on it so a reopened conversation still says so.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What a turn cost and why it stopped.
#[derive(Debug, Clone, Default, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TurnEnd {
    pub turn_id: String,
    pub cost_usd: Option<f64>,
    pub duration_ms: Option<f64>,
    /// The CLI's own word, kept rather than flattened into "failed".
    pub stop_reason: Option<String>,
    pub is_error: bool,
    pub context: Option<crate::frame::Context>,
    /// Messages steered into the turn that the agent never took in, by id.
    pub unheard: Vec<String>,
    /// The model the CLI said it ran on, and the effort it was asked for.
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
}

/// When an answer ended and why: the facts, read into words by the screen.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Ended {
    /// Unix seconds.
    pub at: f64,
    /// The CLI's own word, as [`TurnEnd::stop_reason`].
    pub stop_reason: Option<String>,
    pub is_error: bool,
    /// What the turn failed on, when devpit was told.
    #[serde(default)]
    pub error: Option<String>,
    /// As [`TurnEnd::unheard`]. Absent from a line written before it existed.
    #[serde(default)]
    pub unheard: Vec<String>,
    /// What the turn ran on and spent, for the line under it. Absent from a
    /// line written before it existed.
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub duration_ms: Option<u32>,
    /// How full the context was when it ended.
    #[serde(default)]
    pub tokens: Option<u32>,
}

impl Ended {
    pub fn of(end: &TurnEnd, at: f64) -> Self {
        Self {
            at,
            stop_reason: end.stop_reason.clone(),
            is_error: end.is_error,
            error: None,
            unheard: end.unheard.clone(),
            model: end.model.clone(),
            effort: end.effort.clone(),
            cost_usd: end.cost_usd,
            duration_ms: end.duration_ms.map(|ms| ms as u32),
            tokens: end.context.map(|context| context.used),
        }
    }
}
