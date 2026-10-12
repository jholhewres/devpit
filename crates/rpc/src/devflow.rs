//! Devflow changes only the person makes, as the window asks for them.

use serde::{Deserialize, Serialize};
use specta::Type;

/// What the click does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DevflowChange {
    /// Devflow switched on in the project.
    Enable,
    /// Decisions moved from shadow to enforce in the project's Devflow.
    Enforce,
}

/// One change waiting for the person's click.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowAsk {
    pub id: String,
    pub project_id: String,
    pub project: String,
    pub change: DevflowChange,
    /// Seconds since the epoch.
    pub asked_at: f64,
}

/// A list, so tomorrow's field has somewhere to go.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowAsks {
    pub asks: Vec<DevflowAsk>,
}

/// One card of a Devflow plan, before it is a card.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PlannedCard {
    /// The plan's own id for it, which `depends_on` names.
    pub key: String,
    /// Without the `[devflow] ` the card's title gets.
    pub title: String,
    pub build: String,
    pub criteria: Vec<String>,
    pub files: Vec<String>,
    pub verify: String,
    pub depends_on: Vec<String>,
}

/// A plan previewed for an epic, waiting for the person to create its cards.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowPlan {
    pub epic_id: String,
    pub cards: Vec<PlannedCard>,
    /// The person changed it on the board.
    pub edited: bool,
    /// Seconds since the epoch.
    pub planned_at: f64,
}

/// Response of `devflow.plan_create`: the cards made, in the plan's order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowPlanCreated {
    pub card_ids: Vec<String>,
}

/// How one rubric agrees with the person in a project, from the decisions
/// they labelled by what they did, and the threshold that would agree most.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RubricAccuracy {
    pub rubric: String,
    /// Answered in the last ninety days.
    pub decisions: u32,
    pub labelled: u32,
    /// Labelled, judged pass or fail, and the same as the person.
    pub agreed: u32,
    /// Labelled and left grey.
    pub grey: u32,
    /// `agreed` of the labelled ones not left grey.
    pub accuracy: Option<f64>,
    /// The lowest `min` its yes/no questions have now.
    pub min: Option<f64>,
    pub suggested_min: Option<f64>,
    pub suggested_grey: Option<f64>,
    /// What `suggestedMin` would have agreed on, of the labelled.
    pub suggested_accuracy: Option<f64>,
    /// Why nothing is suggested yet.
    pub note: Option<String>,
}

/// Response of `devflow.calibration`: each rubric the project was judged by.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowCalibration {
    pub project_id: String,
    pub rubrics: Vec<RubricAccuracy>,
}
