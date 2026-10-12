//! Devflow in the MCP server: the three tools that set it up, offered
//! whenever Devflow is not switched off in Settings; the tools of its
//! stages, offered only to a project where the person switched it on; and
//! the watch that tells the client when that changes.
//!
//! What a project gets is the app's answer to `devflow_offer`. The server
//! asks again only when something it can see changed — the project's
//! devflow.yaml, the app's own stamp, or the app restarting — so a session
//! costs a few file reads every couple of seconds, not a question.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::guide;
use crate::mcp::Tool;

/// The file the app touches when what it offers changed without devflow.yaml
/// changing: the person's click, or the switch in Settings.
pub const STAMP: &str = "devflow-stamp";

/// How often the watch looks.
pub(crate) const EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// The notification that makes a client ask for the tools again.
pub(crate) const LIST_CHANGED: &str =
    r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#;

/// Listed even with Devflow off in a project: they are how it is set up.
pub(crate) static SETUP_TOOLS: [Tool; 3] = [
    Tool {
        name: "devpit_devflow_status",
        method: "devflow_status",
        description: "Devflow in this project: its state (no file, disabled, active or invalid, with the line and field of the error), what devpit detected with the file each fact came from (test, E2E and lint commands, Playwright or Cypress, the agent CLIs installed and signed in, a Decisions key, the glossary and ADRs), the checks it recommends, the decisions detection left open, and the current .devpit/devflow/devflow.yaml. Read it first.",
        input: || json!({ "type": "object", "properties": {} }),
    },
    Tool {
        name: "devpit_devflow_setup",
        method: "devflow_setup",
        description: "Set Devflow up in this project. preset recommended: briefing and plan approved by the person, the detected checks each as a script under .devpit/devflow/scripts/, one reviewer with the devflow-security persona, the devflow-done gate, and Decisions in shadow when a key exists; custom: the defaults, with nothing chosen. `set` folds answers in as dotted paths. A preview unless apply is true: the commented YAML and each file it would write. apply: true, only after the person said yes to the preview, writes .devpit/devflow/ (devflow.yaml, the check scripts, a .gitignore for runs/, the reviewer personas and the findings schema). devpit never commits, and Devflow stays off: it switches on when the person clicks the confirmation devpit then shows in the app.",
        input: || json!({ "type": "object", "properties": {
            "preset": { "type": "string", "enum": ["recommended", "custom"], "description": "Where the setup starts. Defaults to recommended." },
            "set": { "type": "object", "description": "Dotted path to value, over the preset, e.g. { \"stages.execution.checks\": [{ \"name\": \"unit\", \"run\": \"make test\", \"report\": \"junit\" }] }." },
            "apply": { "type": "boolean", "description": "Write the files. Only after an explicit yes to the preview." }
        } }),
    },
    Tool {
        name: "devpit_devflow_set",
        method: "devflow_set",
        description: "Change one field of this project's devflow.yaml by its dotted path (review.block_on, review.reviewers, stages.execution.checks, stages.plan.approve): a preview with the value before and after unless apply is true, which writes it only after the person said yes. Works on an invalid file, to fix the field the error names. enabled: true and decisions.mode: enforce are never written here: they are the person's click on a confirmation devpit shows in the app.",
        input: || json!({ "type": "object", "properties": {
            "path": { "type": "string", "description": "The field, e.g. review.block_on or review.reviewers.1 (one past the end adds one)." },
            "value": { "description": "The new value: a string, number, true or false, a list or an object, as the field takes." },
            "apply": { "type": "boolean", "description": "Write it. Only after an explicit yes to the preview." }
        }, "required": ["path", "value"] }),
    },
];

/// The tools only an active project gets: each under the stage that must be
/// switched on for it, or `None` for any. The stages' own cards add theirs.
pub(crate) static STAGED: [(Option<&str>, Tool); 0] = [];

/// What the server offers the project it stands in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Offer {
    /// Switched off in Settings, or no project here: the tools devpit had
    /// before Devflow.
    Off,
    /// The setup tools.
    Setup,
    /// The setup tools and those of the stages switched on.
    Active(Vec<String>),
}

impl Offer {
    /// The app's answer to `devflow_offer`.
    pub(crate) fn of(answer: &Value) -> Offer {
        match answer.get("offer").and_then(Value::as_str) {
            Some("setup") => Offer::Setup,
            Some("active") => Offer::Active(
                answer
                    .get("stages")
                    .and_then(Value::as_array)
                    .map(|stages| {
                        stages
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_owned)
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            _ => Offer::Off,
        }
    }

    /// The Devflow tools offered, after devpit's own.
    pub(crate) fn tools<'a>(&self, staged: &'a [(Option<&'a str>, Tool)]) -> Vec<&'a Tool> {
        match self {
            Offer::Off => Vec::new(),
            Offer::Setup => SETUP_TOOLS.iter().collect(),
            Offer::Active(stages) => SETUP_TOOLS
                .iter()
                .chain(
                    staged
                        .iter()
                        .filter(|(stage, _)| {
                            stage.is_none_or(|stage| stages.iter().any(|on| on == stage))
                        })
                        .map(|(_, tool)| tool),
                )
                .collect(),
        }
    }

    /// The server's `instructions`: devpit's own, and the flow's when it is on.
    pub(crate) fn instructions(&self) -> String {
        match self {
            Offer::Active(_) => format!("{} {}", guide::INSTRUCTIONS, guide::DEVFLOW),
            _ => guide::INSTRUCTIONS.to_owned(),
        }
    }
}

/// A file as the watch last saw it: whether it was there, when it changed,
/// how long it was.
type Seen = Option<(std::time::SystemTime, u64)>;

fn seen(path: &Path) -> Seen {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// What the server offers, and what it last saw of the files that would say
/// the offer changed.
pub(crate) struct Watch {
    pub offer: Offer,
    root: PathBuf,
    file: Option<PathBuf>,
    looked: Option<Vec<Seen>>,
}

impl Watch {
    pub(crate) fn new(root: &Path) -> Self {
        Watch {
            offer: Offer::Off,
            root: root.to_path_buf(),
            file: None,
            looked: None,
        }
    }

    fn now(&self) -> Vec<Seen> {
        let mut files = vec![self.root.join("hook-endpoint"), self.root.join(STAMP)];
        files.extend(self.file.clone());
        files.iter().map(|path| seen(path)).collect()
    }

    /// What the files look like now, when it differs from the last look:
    /// time to ask the app again.
    pub(crate) fn due(&self) -> Option<Vec<Seen>> {
        let now = self.now();
        (self.looked.as_ref() != Some(&now)).then_some(now)
    }

    /// Takes the app's answer, given after the look `looked`, and says the
    /// notification to send when the offer changed. A question the app did
    /// not answer keeps the offer as it was.
    pub(crate) fn settle(
        &mut self,
        looked: Vec<Seen>,
        answer: Result<Value, String>,
    ) -> Option<&'static str> {
        self.looked = Some(looked);
        let answer = answer.ok()?;
        let file = answer
            .get("watch")
            .and_then(Value::as_str)
            .map(PathBuf::from);
        if file.is_some() && file != self.file {
            self.file = file;
            // Seen again with the file in the look, or the next one is "due" for nothing.
            self.looked = Some(self.now());
        }
        let offer = Offer::of(&answer);
        if offer == self.offer {
            return None;
        }
        self.offer = offer;
        Some(LIST_CHANGED)
    }
}

#[cfg(test)]
#[path = "devflow_tests.rs"]
mod tests;
