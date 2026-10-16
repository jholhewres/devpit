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
use crate::listing::Seat;
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
        description: "Devflow in this project: its state (no file, disabled, active or invalid, with the line and field of the error), what devpit detected with the file each fact came from (test, E2E and lint commands, Playwright or Cypress, the agent CLIs installed and signed in, a Decisions key, the glossary and ADRs), the checks it recommends, the decisions detection left open, the current .devpit/devflow/devflow.yaml, and `next`, the step that comes next. Read it first.",
        input: || json!({ "type": "object", "properties": {} }),
    },
    Tool {
        name: "devpit_devflow_setup",
        method: "devflow_setup",
        description: "Set Devflow up here. preset recommended (the default): briefing and plan approved by the person, the detected checks that can run here as scripts (one whose program is missing, or a make target whose recipe calls one, is listed under `unavailable` with why), a devflow-security reviewer, the devflow-done gate, Decisions in shadow when a key exists; custom: the bare defaults. `set` folds answers in by dotted path. A preview of the YAML and its files unless apply: true, sent only after the person's yes, which writes only the setup last previewed: one that changed since comes back as a new preview with applied: false. Never commits, and Devflow stays off until the person clicks devpit's confirmation.",
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

/// The review's tool.
pub const REVIEW_TOOL: &str = "devpit_devflow_review";

/// The gate's tool, which devpit's own stop hooks call too.
pub const GATE_TOOL: &str = "devpit_devflow_gate";

/// Where Claude Code reads how many stops in a row a hook may send back;
/// the gate follows it (`devpit_devflow::gate::CAP_ENV`).
pub const CAP_ENV: &str = "CLAUDE_CODE_STOP_HOOK_BLOCK_CAP";

/// What only this process can add to a stop hook's call to the gate: the
/// pane the session runs in and the cap its CLI was given, both in the
/// environment the CLI started this server with.
pub(crate) fn hooked(
    tool: &str,
    mut arguments: Value,
    env: &dyn Fn(&str) -> Option<String>,
) -> Value {
    if tool != GATE_TOOL || arguments.get("event").is_none() {
        return arguments;
    }
    for (key, variable) in [("pane", crate::hook::PANE_ENV), ("cap", CAP_ENV)] {
        if let Some(value) = env(variable).filter(|value| !value.is_empty()) {
            if arguments.get(key).is_none() {
                arguments[key] = json!(value);
            }
        }
    }
    arguments
}

/// What `devpit agent devflow-context` asks the app, from the payload a
/// session-start hook pipes in.
pub(crate) fn context_params(payload: &Value, pane: &str, agent: &str) -> Value {
    json!({
        "pane": pane,
        "session": payload.get("session_id").and_then(Value::as_str).unwrap_or_default(),
        "agent": agent,
    })
}

/// The session-start hook's reply, as Claude Code and Codex both read it;
/// nothing when the app had nothing to say.
pub(crate) fn context_reply(answer: &Value) -> Option<String> {
    let said = answer.get("context")?.as_str()?;
    Some(
        json!({ "hookSpecificOutput": { "hookEventName": "SessionStart", "additionalContext": said } })
            .to_string(),
    )
}

/// The tools only an active project gets: each under the stages one of which
/// must be switched on for it, or none for any. The stages' own cards add theirs.
pub(crate) static STAGED: [(&[&str], Tool); 8] = [
    (
        &[],
        Tool {
            name: crate::skills::TOOL,
            method: "devflow_skill",
            description: "The Devflow skill to follow for a stage, as this project resolves it: its own .devpit/devflow/skills/<name>/SKILL.md, else the person's, else devpit's built-in one. Call it before each stage and follow what it returns; devflow-setup is for changing the setup. A stage switched off in devflow.yaml has no skill.",
            input: || json!({ "type": "object", "properties": {
                "name": { "type": "string", "enum": crate::skills::NAMES, "description": "The skill: devflow-<stage>, or devflow-setup." }
            }, "required": ["name"] }),
        },
    ),
    // The context stage writes its summary onto the brief.
    (
        &["briefing", "context"],
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
        &["plan"],
        Tool {
            name: "devpit_devflow_plan",
            method: "devflow_plan",
            description: "Devflow: an epic's plan as its child cards, each titled '[devflow] <title>', its body the build, the criteria as checkboxes under ## Acceptance criteria, the likely files and the verify command, and its depends_on as blocking edges the board shows. Without apply it previews and creates nothing: the plan waits on the epic, where the person can move, cut or edit it and create the cards with a click. apply: true creates the cards, only after the person's explicit yes to the preview, and only the plan the person last saw: cards that differ become a new preview. When the person already approved on the board, the answer says so with the card ids. An orchestrator plans another project's epic with `project`, shows the person the preview, and applies with `approval` quoting their yes; apply without `cards` applies the preview as it stands. Once the cards exist, `merge: { from, into }` says a card's work went into another — it finishes on that card's evidence — and `cut` drops a card from the plan, closing it without evidence; either releases the cards waiting on it. Without apply it previews what changes; apply: true only after the person's explicit yes.",
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
                "apply": { "type": "boolean", "description": "Create the cards. Only after the person's explicit yes to the preview." },
                "approval": { "type": "object", "description": "Orchestrator only, with apply: the yes the person gave you, passed on.", "properties": {
                    "by": { "type": "string", "enum": ["orchestrator"] },
                    "quote": { "type": "string", "description": "The person's message saying yes, in their words." }
                }, "required": ["quote"] },
                "project": { "type": "string", "description": "Orchestrator only: the epic's project, by id or name." },
                "merge": { "type": "object", "description": "Instead of epic and cards: a created card whose work went into another.", "properties": {
                    "from": { "type": "string", "description": "The card merged, by id or #number." },
                    "into": { "type": "string", "description": "The card that holds its work now." }
                }, "required": ["from", "into"] },
                "cut": { "type": "string", "description": "Instead of epic and cards: a created card dropped from the plan, by id or #number." }
            } }),
        },
    ),
    (
        &["execution"],
        Tool {
            name: "devpit_devflow_evidence",
            method: "devflow_evidence",
            description: "Devflow: run a card's checks from .devpit/devflow/devflow.yaml in the checkout you stand in, each through devpit's step runner with the card in its environment; keep the reports, screenshots and video in devpit's own folder for the project, never the repository (the newest 5 runs per card; the answer's `run` says where); attach them to the card as artifacts with a comment linking each; and answer with a short summary: each check passed, failed, inconclusive or not run (a check set to write a report that left none devpit reads is inconclusive; one set to `report: none` is judged by its exit code, 127 meaning its command was not found; one whose `requires` probe failed is not run, and checks naming the same `exclusive` lock never overlap), its counts, the failures, and the paths. It records each criterion's tests and how they ended, and the red tests you saw and whether they are green now. Leave `checks` out to run all of them. Takes as long as the checks do.",
            input: || json!({ "type": "object", "properties": {
                "card": { "type": "string", "description": "The card's id." },
                "checks": { "type": "array", "items": { "type": "string" }, "description": "Checks by their name in devflow.yaml. Leave out to run all." },
                "criteria": { "type": "array", "items": { "type": "object", "properties": {
                    "criterion": { "type": "string", "description": "The checkbox text, verbatim." },
                    "tests": { "type": "array", "items": { "type": "string" }, "description": "The tests that prove it, as the runner names them, e.g. tests/test_cli.py::test_rename_changes_the_title." }
                }, "required": ["criterion", "tests"] } },
                "red": { "type": "array", "items": { "type": "object", "properties": {
                    "test": { "type": "string" },
                    "failure": { "type": "string", "description": "The failing line you saw, secrets redacted." }
                }, "required": ["test", "failure"] } }
            }, "required": ["card"] }),
        },
    ),
    (
        &["review"],
        Tool {
            name: REVIEW_TOOL,
            method: "devflow_review",
            description: "Devflow: review a card's change with devflow.yaml's review.reviewers, each headless on the person's own agent CLI (one missing or signed out is skipped), from one packet: the diff from the card's base to HEAD (from a previous run's HEAD with `previous`), the card, its criteria and the persona. Writes findings.json for the gate and comments the count on the card. Answers the verdict by review.block_on, each reviewer's time and cost, and `pending`: personas no reviewer covered — review those yourself, then call again with `run` and `reports`. Takes as long as the reviewers do.",
            input: || json!({ "type": "object", "properties": {
                "card": { "type": "string", "description": "The card's id." },
                "base": { "type": "string", "description": "The branch or commit the diff starts from, when it is not the card's own base. Usually left out." },
                "previous": { "type": "string", "description": "After fixing a review's blocking findings: that review's run. The reviewers read only what changed since its HEAD, with its blocking findings to check again; its other findings are carried over. Leave out `base` and `run`." },
                "run": { "type": "string", "description": "The run a first call answered, which `reports` complete." },
                "reports": { "type": "array", "items": { "type": "object" }, "description": "With `run`: one findings object for each persona in `pending` that you reviewed, as devflow-findings.schema.json beside the run's packet describes." }
            }, "required": ["card"] }),
        },
    ),
    (
        &["completion"],
        Tool {
            name: GATE_TOOL,
            method: "devflow_gate",
            description: "Devflow: whether a card may be called done — its newest evidence passed after the last code change, each criterion maps to passing tests, its E2E spec kept a screenshot, findings.json has nothing in review.block_on, each gated stage passed; then, with Decisions, the devflow-done rubric, which holds the card only in enforce. Comments on the card; answers passed or failed with what is missing and the stage that fixes it. Call it with `card` before devpit_finish_card, which is refused until it passes.",
            input: || json!({ "type": "object", "properties": {
                "card": { "type": "string", "description": "The card's id." }
            } }),
        },
    ),
    (
        &[],
        Tool {
            name: "devpit_devflow_stage",
            method: "devflow_stage",
            description: "Devflow: one of the project's own stages, a `uses:` in devflow.yaml beside the built-in ones (e.g. acme/ticket, project/pr). Without `card`: its skill and gate, to follow. With `card`: runs its gate for the card (a command through devpit's step runner in your checkout, or a Decisions rubric), keeps stage-<name>.json in the card's run and comments. devpit_devflow_gate requires each gated stage to pass after the last change. A team layer's command runs only after the person confirms it in the app.",
            input: || json!({ "type": "object", "properties": {
                "stage": { "type": "string", "description": "The stage's name: what follows the last / of its uses." },
                "card": { "type": "string", "description": "The card to pass the stage's gate for. Leave it out to read the stage's skill." }
            }, "required": ["stage"] }),
        },
    ),
    (
        &["plan"],
        Tool {
            name: "devpit_devflow_autopilot",
            method: "devflow_autopilot",
            description: "Devflow: after the person approved an epic's plan, the order its cards would run in, one session per card through the gate, and a request to the person to start it. It starts only on their click in the app, never from this tool; once started it stops for the person at a card others wait on until they checked it, at a stage with approve: person, and at a session waiting on them.",
            input: || json!({ "type": "object", "properties": {
                "epic": { "type": "string", "description": "The epic's card id." }
            }, "required": ["epic"] }),
        },
    ),
];

/// What an orchestrator is offered of Devflow wherever it stands: planning
/// a linked project's epic and passing the person's yes on.
pub(crate) const RELAYED: [&str; 1] = ["devpit_devflow_plan"];

/// The Devflow tools an orchestrator gets that `offered` left out.
pub(crate) fn relayed<'a>(
    staged: &'a [(&'a [&'a str], Tool)],
    offered: &[&Tool],
    seat: &Seat,
) -> Vec<&'a Tool> {
    if seat.orchestrator != Some(true) {
        return Vec::new();
    }
    staged
        .iter()
        .map(|(_, tool)| tool)
        .filter(|tool| RELAYED.contains(&tool.name))
        .filter(|tool| !offered.iter().any(|one| one.name == tool.name))
        .collect()
}

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
    pub(crate) fn tools<'a>(&self, staged: &'a [(&'a [&'a str], Tool)]) -> Vec<&'a Tool> {
        match self {
            Offer::Off => Vec::new(),
            Offer::Setup => SETUP_TOOLS.iter().collect(),
            Offer::Active(stages) => SETUP_TOOLS
                .iter()
                .chain(
                    staged
                        .iter()
                        .filter(|(under, _)| {
                            under.is_empty() || stages.iter().any(|on| under.contains(&on.as_str()))
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
    pub seat: Seat,
    root: PathBuf,
    file: Option<PathBuf>,
    looked: Option<Vec<Seen>>,
}

impl Watch {
    pub(crate) fn new(root: &Path) -> Self {
        Watch {
            offer: Offer::Off,
            seat: Seat::default(),
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
        let (offer, seat) = (Offer::of(&answer), Seat::of(&answer));
        if offer == self.offer && seat == self.seat {
            return None;
        }
        self.offer = offer;
        self.seat = seat;
        Some(&LISTS_CHANGED)
    }
}

#[cfg(test)]
#[path = "devflow_tests.rs"]
mod tests;
