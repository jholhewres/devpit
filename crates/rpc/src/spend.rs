//! What the agents on this machine spent, and how much of a plan is left.
//!
//! Counts are `f64` because they cross into JavaScript numbers.

use serde::{Deserialize, Serialize};
use specta::Type;

/// Tokens by kind.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TokenCounts {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// An agent CLI installation the history reads from.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpendInstallation {
    pub directory: String,
    pub label: String,
    /// False for an installation pointed at another provider: its dollars are
    /// Anthropic's list price for tokens nobody billed that way.
    pub billed: bool,
}

/// One UTC day.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpendDay {
    /// `YYYY-MM-DD`, UTC.
    pub day: String,
    pub cost_usd: f64,
    pub tokens: TokenCounts,
    /// Cost by model, most first.
    pub by_model: Vec<SpendShare>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpendShare {
    pub name: String,
    pub cost_usd: f64,
    pub tokens: f64,
}

/// A model or a project, over the whole range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpendRow {
    pub name: String,
    pub cost_usd: f64,
    pub tokens: TokenCounts,
    pub sessions: u32,
    /// Unix seconds.
    pub last_active: f64,
}

/// A card a session is known to belong to.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpendCard {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpendSession {
    pub session_id: String,
    pub project: String,
    /// The model that wrote most of its tokens.
    pub model: String,
    pub turns: u32,
    pub tokens: TokenCounts,
    pub cost_usd: f64,
    /// Unix seconds.
    pub last_active: f64,
    pub installation: String,
    pub card: Option<SpendCard>,
}

/// Response of `spend.history`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpendHistory {
    pub days: u32,
    pub installations: Vec<SpendInstallation>,
    pub cost_usd: f64,
    pub tokens: TokenCounts,
    pub sessions: u32,
    pub turns: u32,
    pub active_days: u32,
    /// The share of input served from the cache, 0 to 1.
    pub cache_reuse: f64,
    pub daily: Vec<SpendDay>,
    pub models: Vec<SpendRow>,
    pub projects: Vec<SpendRow>,
    /// Most recently active first, at most twenty.
    pub recent: Vec<SpendSession>,
    /// Some of the dollars are list price for an installation not billed by Anthropic.
    pub estimated: bool,
    /// Tokens from models the price table does not know.
    pub unpriced_tokens: f64,
    pub files: u32,
    pub records: u32,
    pub scan_ms: f64,
}

/// One quota window of a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanWindow {
    pub label: String,
    /// 0 to 100.
    pub percent: f64,
    /// Unix seconds.
    pub resets_at: Option<f64>,
}

/// Response of `plan.limits`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlanLimits {
    pub installation: String,
    pub plan: Option<String>,
    pub windows: Vec<PlanWindow>,
    /// Unix seconds.
    pub read_at: f64,
    /// Why there are no windows, said rather than drawn as zero.
    pub problem: Option<String>,
}
