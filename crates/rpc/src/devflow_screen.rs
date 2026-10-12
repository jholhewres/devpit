//! Devflow as the window draws it: a project's devflow.yaml read through the
//! same engine the MCP tools use, and what a card's runs left.

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::devflow::DevflowAsk;

/// Where a project's devflow.yaml stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum DevflowState {
    /// No devflow.yaml: the setup.
    Missing,
    /// Valid, and not switched on by the person here.
    Disabled,
    Active,
    /// Refused, with the line and field; the tools stay off.
    Invalid,
}

/// Why devflow.yaml is refused.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowError {
    pub line: Option<u32>,
    /// A dotted path, empty when the file as a whole is refused.
    pub field: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowCheck {
    pub name: String,
    pub run: String,
    pub report: String,
    pub evidence: Vec<String>,
    /// `file:line` the command was found at, when detection found it.
    pub source: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowReviewer {
    pub agent: String,
    pub persona: String,
    pub model: Option<String>,
}

/// The skill a stage follows, from the layer that has it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowSkill {
    pub name: String,
    /// `project`, `personal` or `built-in`.
    pub layer: String,
    pub file: Option<String>,
    /// Why the layer that has it cannot be used.
    pub refused: Option<String>,
}

/// One stage, in the order the work reaches it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowStage {
    pub name: String,
    pub enabled: bool,
    /// `person` or `none`, for a stage whose result is approved.
    pub approve: Option<String>,
    /// The execution stage's checks.
    pub checks: Vec<DevflowCheck>,
    /// The completion stage's rubric.
    pub gate: Option<String>,
    /// Decisions' rubric for this stage, when one is named.
    pub rubric: Option<String>,
    pub skill: Option<DevflowSkill>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowConfig {
    pub enabled: bool,
    pub stages: Vec<DevflowStage>,
    pub reviewers: Vec<DevflowReviewer>,
    pub block_on: Vec<String>,
    pub consolidate: String,
    pub decisions_use: bool,
    /// `shadow` or `enforce`.
    pub decisions_mode: String,
}

/// A command detection found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowFound {
    /// `test`, `e2e` or `lint`.
    pub kind: String,
    pub command: String,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowCli {
    pub name: String,
    pub installed: bool,
    pub signed_in: Option<bool>,
}

/// What a setup starts from, read off the repository and this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowDetected {
    pub found: Vec<DevflowFound>,
    /// `name (source)` of each E2E tool set up.
    pub e2e_tools: Vec<String>,
    pub clis: Vec<DevflowCli>,
    pub decisions_key: bool,
    pub glossary: Option<String>,
    pub adrs: Vec<String>,
    /// What detection left to the person, one sentence each.
    pub open: Vec<String>,
    /// The recommended setup, before its commands move into scripts.
    pub recommended: DevflowConfig,
}

/// Response of `devflow.screen`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowScreen {
    pub project_id: String,
    pub state: DevflowState,
    /// Off in Settings for every project.
    pub globally_on: bool,
    /// Where devflow.yaml is, whole, to open in an editor.
    pub file: String,
    pub yaml: Option<String>,
    pub config: Option<DevflowConfig>,
    pub error: Option<DevflowError>,
    /// The changes waiting for the person's click here.
    pub waiting: Vec<DevflowAsk>,
    /// Only while there is no file.
    pub detected: Option<DevflowDetected>,
    /// The setup a new project starts from: `recommended` or `custom`.
    pub preset: String,
}

/// One file a setup writes, and what writing it does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowFileChange {
    pub path: String,
    /// `create`, `replace`, `same`, `kept` or `append`.
    pub change: String,
}

/// Response of `devflow.preview`: the commented YAML and its files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowPreview {
    pub yaml: String,
    pub files: Vec<DevflowFileChange>,
}

/// Response of `devflow.skill_text`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowSkillText {
    pub skill: DevflowSkill,
    pub text: String,
}

/// Devflow's preferences on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowPrefs {
    /// Offered to any project at all.
    pub enabled: bool,
    /// The setup a new project starts from: `recommended` or `custom`.
    pub preset: String,
}

/// Response of `devflow.codex_install`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowCodexInstalled {
    pub folder: String,
    pub said: String,
}

/// One reviewer of a card's newest review.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowReviewed {
    pub agent: String,
    pub persona: String,
    /// `answered` or `skipped`.
    pub status: String,
    pub findings: u32,
    pub duration_ms: Option<f64>,
    pub cost_usd: Option<f64>,
    pub reason: Option<String>,
}

/// The card's newest gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowGated {
    pub passed: bool,
    pub run: Option<String>,
    pub at: String,
    pub with_evidence: u32,
    pub criteria: u32,
    pub missing: Vec<String>,
}

/// Response of `devflow.card_evidence`: what a card's Devflow runs left
/// among the project's artifacts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DevflowCardEvidence {
    pub card_id: String,
    pub gate: Option<DevflowGated>,
    pub reviewers: Vec<DevflowReviewed>,
    pub verdict: Option<String>,
    /// The artifacts folder, whole, which `artifacts` are relative to.
    pub folder: String,
    /// The newest run's attached files, as artifact names.
    pub artifacts: Vec<String>,
}
