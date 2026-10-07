//! Decisions: typed questions answered by a model that does not write text.

use serde::{Deserialize, Serialize};
use specta::Type;

/// How Decisions is set up, as the settings show it. The key never comes
/// back: only whether one is kept.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Deciding {
    /// `openrouter` or `vercel`.
    pub provider: String,
    /// Empty is the provider's default.
    pub model: String,
    /// Empty is the provider's default.
    pub url: String,
    pub daily_cap_usd: f64,
    pub key_set: bool,
    /// Since midnight UTC, read off the decision log.
    pub spent_today_usd: f64,
    pub decided_today: u32,
    /// The projects whose state is never sent.
    pub opted_out: Vec<String>,
}

/// One trivial question asked from Settings, to see the key, the address
/// and the model work.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DecisionTried {
    /// The probability it answered, when it answered.
    pub probability: Option<f64>,
    pub latency_ms: Option<u32>,
    pub cost_usd: Option<f64>,
    /// Why it did not answer.
    pub note: Option<String>,
}
