//! `devflow.yaml`, version 1: what a project's Devflow does. Every field but
//! `version` may be left out and takes its default; a field this version does
//! not read is an error, so a typo never passes for a setting.

use std::collections::BTreeMap;

use serde::Serialize;

use crate::node::{self, Key, Node, Value};

/// The schema this build reads and writes.
pub const VERSION: u64 = 1;

/// The stages, in the order the work goes through them.
pub const STAGES: [&str; 6] = [
    "briefing",
    "context",
    "plan",
    "execution",
    "review",
    "completion",
];

/// Where the E2E specs Devflow writes go, unless `stages.execution.e2e_dir` moves them.
pub const E2E_DIR: &str = ".devpit/devflow/e2e";

/// The rubric a card's completion is gated on.
pub const GATE: &str = "devflow-done";

macro_rules! words {
    ($(#[$doc:meta])* $name:ident { $($variant:ident = $word:literal),+ $(,)? }) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub enum $name { $($variant),+ }

        impl $name {
            pub const ALL: &'static [$name] = &[$($name::$variant),+];

            pub fn word(self) -> &'static str {
                match self { $($name::$variant => $word),+ }
            }

            fn of(word: &str) -> Option<Self> {
                Self::ALL.iter().copied().find(|one| one.word() == word)
            }
        }

        impl Serialize for $name {
            fn serialize<S: serde::Serializer>(&self, to: S) -> Result<S::Ok, S::Error> {
                to.serialize_str(self.word())
            }
        }
    };
}

words!(
    /// Who approves a stage's result before anything is made of it.
    Approve { Person = "person", Nobody = "none" }
);
words!(
    /// The report a check writes, which devpit reads as its evidence.
    Report { Junit = "junit", Vitest = "vitest", Jest = "jest", Cargo = "cargo", Nothing = "none" }
);
words!(
    /// What an E2E check keeps besides its report.
    Evidence { Screenshots = "screenshots", Video = "video" }
);
words!(
    /// The agent CLIs a reviewer persona can run in.
    Agent { Claude = "claude", Codex = "codex", Cursor = "cursor", Gemini = "gemini" }
);
words!(
    /// A finding's severity, as the findings schema names them.
    Severity { Critical = "critical", High = "high", Medium = "medium", Low = "low", Info = "info" }
);
words!(
    /// Whether Decisions only logs (`shadow`) or blocks (`enforce`).
    Mode { Shadow = "shadow", Enforce = "enforce" }
);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Config {
    pub version: u64,
    /// Written `true` only by the person's click in the app.
    pub enabled: bool,
    pub stages: Stages,
    pub review: Review,
    pub decisions: Decisions,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stages {
    pub briefing: Approved,
    pub context: Stage,
    pub plan: Approved,
    pub execution: Execution,
    pub review: Stage,
    pub completion: Completion,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Stage {
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Approved {
    pub enabled: bool,
    pub approve: Approve,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Execution {
    pub enabled: bool,
    pub checks: Vec<Check>,
    pub e2e_dir: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Check {
    pub name: String,
    /// One command line, run from the project's root.
    pub run: String,
    pub report: Report,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Completion {
    pub enabled: bool,
    pub gate: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Review {
    /// Which agent CLI runs which persona; a persona not listed is still
    /// reviewed, by the session itself.
    pub reviewers: Vec<Reviewer>,
    pub block_on: Vec<Severity>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Reviewer {
    pub agent: Agent,
    pub persona: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Decisions {
    #[serde(rename = "use")]
    pub use_: bool,
    pub mode: Mode,
    /// Stage to rubric name.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub rubrics: BTreeMap<String, String>,
}

impl Default for Config {
    fn default() -> Self {
        let approved = Approved {
            enabled: true,
            approve: Approve::Person,
        };
        Config {
            version: VERSION,
            enabled: false,
            stages: Stages {
                briefing: approved.clone(),
                context: Stage { enabled: true },
                plan: approved,
                execution: Execution {
                    enabled: true,
                    checks: Vec::new(),
                    e2e_dir: E2E_DIR.to_owned(),
                },
                review: Stage { enabled: true },
                completion: Completion {
                    enabled: true,
                    gate: GATE.to_owned(),
                },
            },
            review: Review {
                reviewers: Vec::new(),
                block_on: vec![Severity::Critical, Severity::High],
            },
            decisions: Decisions {
                use_: false,
                mode: Mode::Shadow,
                rubrics: BTreeMap::new(),
            },
        }
    }
}

impl Config {
    /// The stages switched on, in their order.
    pub fn stages_on(&self) -> Vec<&'static str> {
        let stages = &self.stages;
        let on = [
            stages.briefing.enabled,
            stages.context.enabled,
            stages.plan.enabled,
            stages.execution.enabled,
            stages.review.enabled,
            stages.completion.enabled,
        ];
        STAGES
            .iter()
            .zip(on)
            .filter_map(|(stage, on)| on.then_some(*stage))
            .collect()
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }
}

/// Why a devflow.yaml, or a change to one, is refused: the line when there
/// is one, the field as a dotted path, and what is wrong with it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Invalid {
    pub line: Option<usize>,
    pub field: String,
    pub message: String,
}

impl Invalid {
    pub(crate) fn at(line: Option<usize>, field: &str, message: &str) -> Self {
        Invalid {
            line,
            field: field.to_owned(),
            message: message.to_owned(),
        }
    }
}

impl std::fmt::Display for Invalid {
    fn fmt(&self, to: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self.line, self.field.is_empty()) {
            (Some(line), false) => write!(to, "line {line}, `{}`: {}", self.field, self.message),
            (Some(line), true) => write!(to, "line {line}: {}", self.message),
            (None, false) => write!(to, "`{}`: {}", self.field, self.message),
            (None, true) => write!(to, "{}", self.message),
        }
    }
}

/// The configuration a devflow.yaml text says.
pub fn parse(text: &str) -> Result<Config, Invalid> {
    read(&node::from_yaml(text)?)
}

/// The configuration a JSON value says, checked the same way.
pub fn from_json(value: &serde_json::Value) -> Result<Config, Invalid> {
    read(&node::from_json(value))
}

fn joined(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_owned()
    } else {
        format!("{path}.{name}")
    }
}

/// A map being read: the fields taken so far, so whatever is left over is
/// named as the error it is.
struct Fields<'a> {
    path: String,
    line: Option<usize>,
    items: &'a [(Key, Node)],
    taken: Vec<&'static str>,
}

impl<'a> Fields<'a> {
    fn of(node: &'a Node, path: &str) -> Result<Self, Invalid> {
        let items: &[(Key, Node)] = match &node.value {
            Value::Map(items) => items,
            Value::Null => &[],
            _ => return Err(Invalid::at(node.line(), path, "expected a map of fields")),
        };
        Ok(Fields {
            path: path.to_owned(),
            line: node.line(),
            items,
            taken: Vec::new(),
        })
    }

    fn get(&mut self, name: &'static str) -> Option<(&'a Node, String)> {
        self.taken.push(name);
        self.items
            .iter()
            .find(|(key, node)| key.name == name && node.value != Value::Null)
            .map(|(_, node)| (node, joined(&self.path, name)))
    }

    fn done(self) -> Result<(), Invalid> {
        match self
            .items
            .iter()
            .find(|(key, _)| !self.taken.contains(&key.name.as_str()))
        {
            Some((key, _)) => Err(Invalid::at(
                key.line,
                &joined(&self.path, &key.name),
                &format!(
                    "not a field of {}: it takes {}",
                    if self.path.is_empty() {
                        "devflow.yaml"
                    } else {
                        &self.path
                    },
                    self.taken.join(", ")
                ),
            )),
            None => Ok(()),
        }
    }
}

fn scalar<'a>(node: &'a Node, path: &str, wanted: &str) -> Result<(&'a str, bool), Invalid> {
    match &node.value {
        Value::Scalar(text, plain) => Ok((text, *plain)),
        _ => Err(Invalid::at(
            node.line(),
            path,
            &format!("expected {wanted}"),
        )),
    }
}

fn boolean(field: Option<(&Node, String)>, default: bool) -> Result<bool, Invalid> {
    let Some((node, path)) = field else {
        return Ok(default);
    };
    match scalar(node, &path, "true or false")? {
        ("true" | "True" | "TRUE", true) => Ok(true),
        ("false" | "False" | "FALSE", true) => Ok(false),
        _ => Err(Invalid::at(node.line(), &path, "expected true or false")),
    }
}

fn text(node: &Node, path: &str, most: usize) -> Result<String, Invalid> {
    let (text, _) = scalar(node, path, "a word or a line of text")?;
    let text = text.trim();
    if text.is_empty() || text.chars().count() > most || text.contains(['\n', '\r']) {
        return Err(Invalid::at(
            node.line(),
            path,
            &format!("expected one line of at most {most} characters"),
        ));
    }
    Ok(text.to_owned())
}

fn word<T: Copy>(
    node: &Node,
    path: &str,
    of: fn(&str) -> Option<T>,
    all: &[T],
    say: fn(T) -> &'static str,
) -> Result<T, Invalid> {
    let (given, _) = scalar(node, path, "a word")?;
    of(given).ok_or_else(|| {
        let words: Vec<&str> = all.iter().map(|one| say(*one)).collect();
        Invalid::at(
            node.line(),
            path,
            &format!("`{given}` is not one of {}", words.join(", ")),
        )
    })
}

fn items<'a>(node: &'a Node, path: &str) -> Result<&'a [Node], Invalid> {
    match &node.value {
        Value::Seq(items) => Ok(items),
        _ => Err(Invalid::at(node.line(), path, "expected a list")),
    }
}

/// A name devpit can make a file of: lowercase letters, digits, `-` and `_`.
pub fn is_name(text: &str) -> bool {
    (1..=40).contains(&text.len())
        && text
            .bytes()
            .next()
            .is_some_and(|first| first.is_ascii_lowercase() || first.is_ascii_digit())
        && text
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_".contains(&byte))
}

fn name(node: &Node, path: &str) -> Result<String, Invalid> {
    let given = text(node, path, 40)?;
    if !is_name(&given) {
        return Err(Invalid::at(
            node.line(),
            path,
            "expected a name of lowercase letters, digits, - and _",
        ));
    }
    Ok(given)
}

/// A folder inside the project, written relative to its root.
fn inside(node: &Node, path: &str) -> Result<String, Invalid> {
    let given = text(node, path, 200)?;
    let outside = given.starts_with('/')
        || given.contains(['\\', ':'])
        || given
            .split('/')
            .any(|part| part == ".." || part.eq_ignore_ascii_case(".git"));
    if outside {
        return Err(Invalid::at(
            node.line(),
            path,
            "expected a folder inside the project, relative to its root",
        ));
    }
    Ok(given.trim_end_matches('/').to_owned())
}

fn read(root: &Node) -> Result<Config, Invalid> {
    if root.value == Value::Null {
        return Err(Invalid::at(
            None,
            "version",
            "devflow.yaml starts with `version: 1`",
        ));
    }
    let defaults = Config::default();
    let mut fields = Fields::of(root, "")?;
    let version = match fields.get("version") {
        Some((node, path)) => match scalar(node, &path, "1")? {
            ("1", true) => VERSION,
            (other, _) => {
                return Err(Invalid::at(
                    node.line(),
                    &path,
                    &format!("this devpit reads version 1, not {other}"),
                ))
            }
        },
        None => {
            return Err(Invalid::at(
                fields.line,
                "version",
                "devflow.yaml starts with `version: 1`",
            ))
        }
    };
    let enabled = boolean(fields.get("enabled"), false)?;
    let stages = match fields.get("stages") {
        Some((node, path)) => stages(node, &path)?,
        None => defaults.stages,
    };
    let review = match fields.get("review") {
        Some((node, path)) => review(node, &path)?,
        None => defaults.review,
    };
    let decisions = match fields.get("decisions") {
        Some((node, path)) => decisions(node, &path)?,
        None => defaults.decisions,
    };
    fields.done()?;
    Ok(Config {
        version,
        enabled,
        stages,
        review,
        decisions,
    })
}

fn stage(field: Option<(&Node, String)>) -> Result<Stage, Invalid> {
    let Some((node, path)) = field else {
        return Ok(Stage { enabled: true });
    };
    let mut fields = Fields::of(node, &path)?;
    let enabled = boolean(fields.get("enabled"), true)?;
    fields.done()?;
    Ok(Stage { enabled })
}

fn approved(field: Option<(&Node, String)>) -> Result<Approved, Invalid> {
    let Some((node, path)) = field else {
        return Ok(Config::default().stages.plan);
    };
    let mut fields = Fields::of(node, &path)?;
    let enabled = boolean(fields.get("enabled"), true)?;
    let approve = match fields.get("approve") {
        Some((node, path)) => word(node, &path, Approve::of, Approve::ALL, Approve::word)?,
        None => Approve::Person,
    };
    fields.done()?;
    Ok(Approved { enabled, approve })
}

fn stages(node: &Node, path: &str) -> Result<Stages, Invalid> {
    let mut fields = Fields::of(node, path)?;
    let briefing = approved(fields.get("briefing"))?;
    let context = stage(fields.get("context"))?;
    let plan = approved(fields.get("plan"))?;
    let execution = match fields.get("execution") {
        Some((node, path)) => execution(node, &path)?,
        None => Config::default().stages.execution,
    };
    let review = stage(fields.get("review"))?;
    let completion = match fields.get("completion") {
        Some((node, path)) => {
            let mut fields = Fields::of(node, &path)?;
            let enabled = boolean(fields.get("enabled"), true)?;
            let gate = match fields.get("gate") {
                Some((node, path)) => name(node, &path)?,
                None => GATE.to_owned(),
            };
            fields.done()?;
            Completion { enabled, gate }
        }
        None => Config::default().stages.completion,
    };
    fields.done()?;
    Ok(Stages {
        briefing,
        context,
        plan,
        execution,
        review,
        completion,
    })
}

fn execution(node: &Node, path: &str) -> Result<Execution, Invalid> {
    let mut fields = Fields::of(node, path)?;
    let enabled = boolean(fields.get("enabled"), true)?;
    let mut checks: Vec<Check> = Vec::new();
    if let Some((node, path)) = fields.get("checks") {
        for (index, item) in items(node, &path)?.iter().enumerate() {
            let at = format!("{path}.{index}");
            let made = check(item, &at)?;
            if checks.iter().any(|one| one.name == made.name) {
                return Err(Invalid::at(
                    item.line(),
                    &format!("{at}.name"),
                    &format!("a check called `{}` is already listed", made.name),
                ));
            }
            checks.push(made);
        }
    }
    let e2e_dir = match fields.get("e2e_dir") {
        Some((node, path)) => inside(node, &path)?,
        None => E2E_DIR.to_owned(),
    };
    fields.done()?;
    Ok(Execution {
        enabled,
        checks,
        e2e_dir,
    })
}

fn check(node: &Node, path: &str) -> Result<Check, Invalid> {
    let mut fields = Fields::of(node, path)?;
    let line = fields.line;
    let name = match fields.get("name") {
        Some((node, path)) => name(node, &path)?,
        None => {
            return Err(Invalid::at(
                line,
                &joined(path, "name"),
                "a check needs a name",
            ))
        }
    };
    let run = match fields.get("run") {
        Some((node, path)) => text(node, &path, 1000)?,
        None => {
            return Err(Invalid::at(
                line,
                &joined(path, "run"),
                "a check needs a command to run",
            ))
        }
    };
    let report = match fields.get("report") {
        Some((node, path)) => word(node, &path, Report::of, Report::ALL, Report::word)?,
        None => Report::Nothing,
    };
    let mut evidence = Vec::new();
    if let Some((node, path)) = fields.get("evidence") {
        for (index, item) in items(node, &path)?.iter().enumerate() {
            let kept = word(
                item,
                &format!("{path}.{index}"),
                Evidence::of,
                Evidence::ALL,
                Evidence::word,
            )?;
            if !evidence.contains(&kept) {
                evidence.push(kept);
            }
        }
    }
    fields.done()?;
    Ok(Check {
        name,
        run,
        report,
        evidence,
    })
}

fn review(node: &Node, path: &str) -> Result<Review, Invalid> {
    let mut fields = Fields::of(node, path)?;
    let mut reviewers = Vec::new();
    if let Some((node, path)) = fields.get("reviewers") {
        for (index, item) in items(node, &path)?.iter().enumerate() {
            reviewers.push(reviewer(item, &format!("{path}.{index}"))?);
        }
    }
    let block_on = match fields.get("block_on") {
        Some((node, path)) => {
            let mut severities = Vec::new();
            for (index, item) in items(node, &path)?.iter().enumerate() {
                let severity = word(
                    item,
                    &format!("{path}.{index}"),
                    Severity::of,
                    Severity::ALL,
                    Severity::word,
                )?;
                if !severities.contains(&severity) {
                    severities.push(severity);
                }
            }
            severities
        }
        None => Config::default().review.block_on,
    };
    fields.done()?;
    Ok(Review {
        reviewers,
        block_on,
    })
}

fn reviewer(node: &Node, path: &str) -> Result<Reviewer, Invalid> {
    let mut fields = Fields::of(node, path)?;
    let line = fields.line;
    let agent = match fields.get("agent") {
        Some((node, path)) => word(node, &path, Agent::of, Agent::ALL, Agent::word)?,
        None => {
            return Err(Invalid::at(
                line,
                &joined(path, "agent"),
                "a reviewer needs an agent",
            ))
        }
    };
    let persona = match fields.get("persona") {
        Some((node, path)) => name(node, &path)?,
        None => {
            return Err(Invalid::at(
                line,
                &joined(path, "persona"),
                "a reviewer needs a persona",
            ))
        }
    };
    let model = match fields.get("model") {
        Some((node, path)) => Some(text(node, &path, 100)?),
        None => None,
    };
    fields.done()?;
    Ok(Reviewer {
        agent,
        persona,
        model,
    })
}

fn decisions(node: &Node, path: &str) -> Result<Decisions, Invalid> {
    let mut fields = Fields::of(node, path)?;
    let use_ = boolean(fields.get("use"), false)?;
    let mode = match fields.get("mode") {
        Some((node, path)) => word(node, &path, Mode::of, Mode::ALL, Mode::word)?,
        None => Mode::Shadow,
    };
    let mut rubrics = BTreeMap::new();
    if let Some((node, path)) = fields.get("rubrics") {
        let Value::Map(items) = &node.value else {
            return Err(Invalid::at(
                node.line(),
                &path,
                "expected a map of stage to rubric",
            ));
        };
        for (key, rubric) in items {
            let at = joined(&path, &key.name);
            if !STAGES.contains(&key.name.as_str()) {
                return Err(Invalid::at(
                    key.line,
                    &at,
                    &format!("not a stage: the stages are {}", STAGES.join(", ")),
                ));
            }
            rubrics.insert(key.name.clone(), name(rubric, &at)?);
        }
    }
    fields.done()?;
    Ok(Decisions {
        use_,
        mode,
        rubrics,
    })
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
