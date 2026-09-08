//! The board vocabulary: columns, cards, steps and runs.
//!
//! A column is data. Nothing here — and nothing that consumes it — may match
//! on a column's name: the person renames them, and a screen keyed to
//! "review" breaks the moment they do.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What a column runs when a card arrives in it.
///
/// Three kinds because they behave in opposite ways, and the difference is
/// visible on screen: only `Session` takes the terminal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// One headless turn. Returns schema-validated JSON, takes no terminal.
    Agent,
    /// A session you drive. This one takes the target terminal.
    Session,
    /// A command of yours: tests, a build, a deploy.
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: String,
    pub kind: StepKind,
    pub name: String,
    /// Shaped by `kind`. Opaque here on purpose: the contract would otherwise
    /// have to grow a variant every time a step learns an option.
    pub config: String,
    /// A deploy has no undo, so it is confirmed rather than fired by a drag.
    pub irreversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub id: String,
    pub name: String,
    pub position: i32,
    /// `null` means the column runs nothing, which is a column doing its job.
    pub step: Option<Step>,
}

/// How a run ended, or that it has not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum RunState {
    Running,
    Ok,
    Failed,
    Cancelled,
}

/// One execution of a step, and what it cost.
///
/// The cost is on the record rather than derived later: "the agent is doing
/// something" stops being an acceptable answer once the card can say what it
/// spent.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    pub id: String,
    pub step_id: String,
    pub step_name: String,
    pub state: RunState,
    pub output: Option<String>,
    pub exit_code: Option<i32>,
    pub cost_usd: Option<f64>,
    /// `f64` and not `i64` throughout, because this crosses into a JavaScript
    /// number and that is what a JavaScript number is — see
    /// `Commit::committed_at` for the same reason stated once.
    pub duration_ms: Option<f64>,
    pub started_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub id: String,
    pub column_id: String,
    pub title: String,
    pub body: String,
    pub position: i32,
    pub worktree_path: Option<String>,
    /// What every run of this card has cost, added up.
    pub cost_usd: f64,
    /// Most recent first.
    pub runs: Vec<Run>,
}

/// Response of `board.get`.
///
/// An object rather than a bare list of columns: the board will grow a field,
/// and a bare list has nowhere to put it.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Board {
    pub project_id: String,
    pub columns: Vec<Column>,
    pub cards: Vec<Card>,
    pub steps: Vec<Step>,
}

/// Response of `board.column_delete` when the column still holds cards.
///
/// Refusing is the answer, but refusing without saying how many would leave
/// the screen asking a question it cannot phrase.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDeleted {
    pub deleted: bool,
    pub cards_in_the_way: u32,
}

/// Response of `card.create` and `card.move`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CardChanged {
    pub card: Card,
    /// The run this move started, when the column it landed in has a step.
    pub started: Option<Run>,
}
