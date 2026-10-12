//! Devflow in the MCP server: the three tools that set it up, offered
//! whenever Devflow is not switched off in Settings; the tools of its
//! stages and its skills, offered only to a project where the person
//! switched it on; and the watch that tells the client when that changes.
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

/// The notifications that make a client ask for the tools, the skills'
/// resources and the stages' prompts again.
pub(crate) const LISTS_CHANGED: [&str; 3] = [
    r#"{"jsonrpc":"2.0","method":"notifications/tools/list_changed"}"#,
    r#"{"jsonrpc":"2.0","method":"notifications/resources/list_changed"}"#,
    r#"{"jsonrpc":"2.0","method":"notifications/prompts/list_changed"}"#,
];

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
pub(crate) static STAGED: [(Option<&str>, Tool); 3] = [
    (
        None,
        Tool {
            name: crate::skills::TOOL,
            method: "devflow_skill",
            description: "The Devflow skill to follow for a stage, as this project resolves it: its own .devpit/devflow/skills/<name>/SKILL.md, else the person's, else devpit's built-in one. Call it before each stage and follow what it returns; devflow-setup is for changing the setup. A stage switched off in devflow.yaml has no skill.",
            input: || json!({ "type": "object", "properties": {
                "name": { "type": "string", "enum": crate::skills::NAMES, "description": "The skill: devflow-<stage>, or devflow-setup." }
            }, "required": ["name"] }),
        },
    ),
    (
        Some("briefing"),
        Tool {
            name: "devpit_devflow_brief",
            method: "devflow_brief",
            description: "Devflow: write a brief onto the board, as the epic card titled '[devflow] Brief: <title>' — goal, for whom, success, out of scope, constraints and decisions. Only after the person's explicit yes to your restate. Without `card` it creates the epic; with `card` it writes onto that card, renaming it when `title` is given. `context` (the context stage's summary, a markdown ## Context section) replaces the epic's context. Fields left out stay as they were. Answers with the card id.",
            input: || json!({ "type": "object", "properties": {
                "card": { "type": "string", "description": "An existing card to write the brief onto: the epic, or the card the work started from." },
                "title": { "type": "string", "description": "The short title; the card becomes '[devflow] Brief: <title>'. Needed for a new epic." },
                "goal": { "type": "string" },
                "for_whom": { "type": "string" },
                "success": { "type": "string", "description": "An observable check: a command, a screen, a number." },
                "out_of_scope": { "type": "array", "items": { "type": "string" } },
                "constraints": { "type": "array", "items": { "type": "string" } },
                "decisions": { "type": "array", "items": { "type": "string" } },
                "context": { "type": "string", "description": "The context stage's summary, starting with ## Context." }
            } }),
        },
    ),
    (
        Some("plan"),
        Tool {
            name: "devpit_devflow_plan",
            method: "devflow_plan",
            description: "Devflow: an epic's plan as its child cards, each titled '[devflow] <title>', its body the build, the criteria as checkboxes under ## Acceptance criteria, the likely files and the verify command, and its depends_on as blocking edges the board shows. Without apply it previews and creates nothing: the plan waits on the epic, where the person can move, cut or edit it and create the cards with a click. apply: true creates the cards, only after the person's explicit yes to the preview, and only the plan the person last saw: cards that differ become a new preview. When the person already approved on the board, the answer says so with the card ids.",
            input: || json!({ "type": "object", "properties": {
                "epic": { "type": "string", "description": "The epic's card id." },
                "cards": { "type": "array", "items": { "type": "object", "properties": {
                    "key": { "type": "string", "description": "A short local id, e.g. s1, that depends_on refers to." },
                    "title": { "type": "string", "description": "The behaviour, in the glossary's words; the tool adds '[devflow] '." },
                    "build": { "type": "string", "description": "What works end to end when it lands, one paragraph." },
                    "criteria": { "type": "array", "items": { "type": "string" }, "description": "Testable conditions; they become checkboxes." },
                    "files": { "type": "array", "items": { "type": "string" }, "description": "The files it likely touches." },
                    "verify": { "type": "string", "description": "One command that proves the criteria." },
                    "depends_on": { "type": "array", "items": { "type": "string" }, "description": "The keys of the cards that block it." }
                }, "required": ["key", "title"] } },
                "apply": { "type": "boolean", "description": "Create the cards. Only after the person's explicit yes to the preview." }
            }, "required": ["epic", "cards"] }),
        },
    ),
];

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

    /// The server's `instructions`: devpit's own, and the flow's when it is
    /// on — first, because Codex reads only the start.
    pub(crate) fn instructions(&self) -> String {
        match self {
            Offer::Active(_) => format!("{}\n\n{}", guide::DEVFLOW, guide::INSTRUCTIONS),
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
    /// notifications to send when the offer changed. A question the app did
    /// not answer keeps the offer as it was.
    pub(crate) fn settle(
        &mut self,
        looked: Vec<Seen>,
        answer: Result<Value, String>,
    ) -> Option<&'static [&'static str]> {
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
        Some(&LISTS_CHANGED)
    }
}

#[cfg(test)]
#[path = "devflow_tests.rs"]
mod tests;
