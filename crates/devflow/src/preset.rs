//! The two starting points of a setup, and a change of one field.
//!
//! `recommended` is the configuration devpit would choose from what it
//! detected; `custom` is the schema's defaults with nothing chosen. Either is
//! then adjusted by dotted paths — the answers to what detection left open —
//! and the result is checked by the same reader as a file.

use serde::Serialize;
use serde_json::Value;

use crate::config::{self, Agent, Check, Config, Evidence, Invalid, Mode, Reviewer};
use crate::detect::Detected;

/// The persona the recommended reviewer takes.
pub const SECURITY: &str = "devflow-security";

/// The agent CLIs, in the order a reviewer is picked from them.
const REVIEWERS: [Agent; 4] = [Agent::Claude, Agent::Codex, Agent::Cursor, Agent::Gemini];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Recommended,
    Custom,
}

impl Preset {
    pub fn of(word: &str) -> Option<Self> {
        match word {
            "recommended" => Some(Preset::Recommended),
            "custom" => Some(Preset::Custom),
            _ => None,
        }
    }

    pub fn word(self) -> &'static str {
        match self {
            Preset::Recommended => "recommended",
            Preset::Custom => "custom",
        }
    }
}

/// A check the setup proposes, with where its command was found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Proposed {
    pub name: String,
    pub run: String,
    pub report: config::Report,
    pub source: Option<String>,
    pub evidence: Vec<Evidence>,
}

/// The checks devpit would configure: the first test command, the E2E check
/// and the first lint command it found. With Playwright the E2E check runs
/// Devflow's specs through the config devpit writes, keeping screenshots and
/// video; the project's own E2E command, when there is one, is still called,
/// as `e2e-suite`.
pub fn checks(detected: &Detected) -> Vec<Proposed> {
    let found = |name: &str, found: &crate::detect::Found, evidence: Vec<Evidence>| Proposed {
        name: name.to_owned(),
        run: found.command.clone(),
        report: found.report,
        source: Some(found.source.clone()),
        evidence,
    };
    let own_e2e = detected.e2e.first();
    let devflows = detected.e2e_tools.iter().find_map(|tool| {
        Some(Proposed {
            name: "e2e".to_owned(),
            run: tool.run.clone()?,
            report: config::Report::Junit,
            source: Some(tool.source.clone()),
            evidence: vec![Evidence::Screenshots, Evidence::Video],
        })
    });
    let e2e = match devflows {
        Some(devflows) => [
            Some(devflows),
            own_e2e.map(|own| found("e2e-suite", own, Vec::new())),
        ],
        None => {
            let screenshots = if detected.e2e_tools.is_empty() {
                Vec::new()
            } else {
                vec![Evidence::Screenshots]
            };
            [own_e2e.map(|own| found("e2e", own, screenshots)), None]
        }
    };
    std::iter::once(
        detected
            .test
            .first()
            .map(|one| found("unit", one, Vec::new())),
    )
    .chain(e2e)
    .chain(std::iter::once(
        detected
            .lint
            .first()
            .map(|one| found("lint", one, Vec::new())),
    ))
    .flatten()
    .collect()
}

/// What `preset` starts from, given what was detected.
pub fn starting(preset: Preset, detected: &Detected) -> Config {
    let mut config = Config::default();
    if preset == Preset::Custom {
        return config;
    }
    config.stages.execution.checks = checks(detected)
        .into_iter()
        .map(|proposed| Check {
            evidence: proposed.evidence,
            name: proposed.name,
            run: proposed.run,
            report: proposed.report,
        })
        .collect();
    let reviewer = REVIEWERS.into_iter().find(|agent| {
        detected
            .clis
            .iter()
            .any(|cli| cli.name == *agent && cli.installed && cli.signed_in != Some(false))
    });
    config.review.reviewers = reviewer
        .map(|agent| Reviewer {
            agent,
            persona: SECURITY.to_owned(),
            model: None,
        })
        .into_iter()
        .collect();
    config.decisions.use_ = detected.decisions_key && !detected.decisions_opted_out;
    config.decisions.mode = Mode::Shadow;
    config
}

/// `config` with the value at the dotted `path` replaced. Checked like a file.
pub fn with_set(config: &Config, path: &str, value: &Value) -> Result<Config, Invalid> {
    config::from_json(&set_path(config.to_json(), path, value)?)
}

/// `whole` with the value at the dotted `path` replaced: a field, a list item
/// by index (one past the end adds one), or a whole map or list.
pub(crate) fn set_path(mut whole: Value, path: &str, value: &Value) -> Result<Value, Invalid> {
    let parts: Vec<&str> = path.split('.').map(str::trim).collect();
    if parts.iter().any(|part| part.is_empty()) {
        return Err(Invalid::at(None, path, "a path is fields joined by dots"));
    }
    if parts == ["version"] {
        return Err(Invalid::at(None, path, "the version is devpit's to write"));
    }
    if !whole.is_object() {
        whole = Value::Object(Default::default());
    }
    let mut at = &mut whole;
    for (index, part) in parts.iter().enumerate() {
        let last = index + 1 == parts.len();
        at = match at {
            Value::Object(fields) => {
                if last {
                    fields.insert((*part).to_owned(), value.clone());
                    break;
                }
                fields
                    .entry((*part).to_owned())
                    .or_insert_with(|| Value::Object(Default::default()))
            }
            Value::Array(items) => {
                let shown = parts[..index].join(".");
                let Some(slot) = part
                    .parse::<usize>()
                    .ok()
                    .filter(|slot| *slot <= items.len())
                else {
                    return Err(Invalid::at(
                        None,
                        path,
                        &format!(
                            "{shown} has {} items: index 0 to {}",
                            items.len(),
                            items.len()
                        ),
                    ));
                };
                if slot == items.len() {
                    items.push(Value::Object(Default::default()));
                }
                if last {
                    items[slot] = value.clone();
                    break;
                }
                &mut items[slot]
            }
            _ => {
                return Err(Invalid::at(
                    None,
                    path,
                    &format!("{} holds a value, not fields", parts[..index].join(".")),
                ))
            }
        };
    }
    Ok(whole)
}

/// The value at a dotted path, or null.
pub fn value_at<'a>(whole: &'a Value, path: &str) -> &'a Value {
    path.split('.')
        .try_fold(whole, |at, part| match at {
            Value::Object(fields) => fields.get(part),
            Value::Array(items) => items.get(part.parse::<usize>().ok()?),
            _ => None,
        })
        .unwrap_or(&Value::Null)
}

/// One field that differs, by dotted path. Lists are compared whole.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Change {
    pub path: String,
    pub before: Value,
    pub after: Value,
}

/// What changes from `before` to `after`.
pub fn changes(before: Option<&Config>, after: &Config) -> Vec<Change> {
    diff(
        &before.map(Config::to_json).unwrap_or(Value::Null),
        &after.to_json(),
    )
}

/// [`changes`], between two JSON forms.
pub(crate) fn diff(before: &Value, after: &Value) -> Vec<Change> {
    let mut said = Vec::new();
    walk("", before, after, &mut said);
    said
}

fn walk(path: &str, before: &Value, after: &Value, said: &mut Vec<Change>) {
    // A map that is not there yet is an empty one: its fields are each new.
    let empty = Value::Object(Default::default());
    let (before, after) = match (before, after) {
        (Value::Null, Value::Object(_)) => (&empty, after),
        (Value::Object(_), Value::Null) => (before, &empty),
        _ => (before, after),
    };
    if let (Value::Object(old), Value::Object(new)) = (before, after) {
        let mut names: Vec<&String> = old.keys().chain(new.keys()).collect();
        names.sort();
        names.dedup();
        for name in names {
            let deeper = if path.is_empty() {
                name.clone()
            } else {
                format!("{path}.{name}")
            };
            walk(
                &deeper,
                old.get(name).unwrap_or(&Value::Null),
                new.get(name).unwrap_or(&Value::Null),
                said,
            );
        }
        return;
    }
    if before != after {
        said.push(Change {
            path: path.to_owned(),
            before: before.clone(),
            after: after.clone(),
        });
    }
}

#[cfg(test)]
#[path = "preset_tests.rs"]
mod tests;
