//! What the terminals are costing.

use serde::{Deserialize, Serialize};
use specta::Type;

/// One pane, and the whole process tree under it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PaneCost {
    pub pane_id: String,
    /// The agent's name when it is one, the executable's otherwise.
    pub label: String,
    pub agent: Option<String>,
    /// Kibibytes, proportional where the kernel would say — see `proportional`.
    ///
    /// 32 bits because the contract crosses into JavaScript, which holds
    /// integers exactly only to 2^53 — so specta refuses a `u64` outright
    /// rather than letting a number arrive quietly wrong. Four tebibytes is
    /// past anything a terminal will hold.
    pub memory_kb: u32,
    /// Tenths of a percent, since the last time this was asked. Zero for a
    /// pane asked about once: a rate from one sample is not a rate.
    ///
    /// Tenths and not a float, because specta types an `f64` as `number |
    /// null` — a float can be NaN, which JSON has no word for. An integer
    /// crosses the wire meaning exactly what it says, and the rounding
    /// happens once, here, rather than in Rust and again in TypeScript.
    pub cpu_tenths: u32,
    /// How many processes the tree holds. An agent with fifteen is worth
    /// knowing about even when the memory looks ordinary.
    pub processes: u32,
}

/// Every pane of a project, and the total.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub memory_kb: u32,
    /// Tenths of a percent. See `PaneCost::cpu_tenths`.
    pub cpu_tenths: u32,
    /// Whether shared pages were divided among the processes sharing them.
    ///
    /// False means at least one process could only be read as resident, and
    /// the total is therefore an overcount — measured at 44% on one machine.
    /// Said rather than hidden: a number that might be half wrong has to
    /// arrive labelled.
    pub proportional: bool,
    pub panes: Vec<PaneCost>,
}
