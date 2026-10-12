//! Decisions, asked by an agent: `decide` with questions of its own, and
//! `rubric_run` with a rubric from the project or one devpit ships.
//!
//! Read-only for the board. Logged like every decision, under the gate
//! `mcp.<agent>`, and refused with a sentence the agent can repeat when
//! Decisions is off.

use std::path::Path;

use devpit_rpc::Project;
use serde_json::{json, Value};

use crate::deciding::{Decided, GateResult, MOST_STATE};
use crate::rubric::Rubric;

/// The methods here, for the agent API's list.
pub(crate) const METHODS: [&str; 2] = ["decide", "rubric_run"];

/// More than this is a survey, not a decision.
pub(crate) const MOST_QUESTIONS: usize = 20;

/// The most of a rubric file read.
const MOST_RUBRIC_BYTES: u64 = 64 * 1024;

/// The rubric for whether a card is done, which Devflow's gate asks once
/// per criterion (`devflow_gate`).
pub(crate) const DONE: &str = include_str!("rubrics/done.json");

/// The rubrics devpit ships, by name.
const BUILT_IN: [(&str, &str); 1] = [("done", DONE)];

pub(crate) fn respond(
    method: &str,
    project: &Project,
    author: &str,
    params: &Value,
) -> Result<Value, String> {
    let state = params
        .get("state")
        .filter(|state| state.is_string() || state.is_object())
        .cloned()
        .ok_or("pass `state`: the evidence, as text or an object")?;
    let gate = format!("mcp.{author}");
    if method == "decide" {
        let questions = params.get("questions").cloned().unwrap_or(Value::Null);
        counted(&questions)?;
        return answered(crate::deciding::decide(
            Some(&project.id),
            &gate,
            state,
            questions,
        ));
    }
    let name = params
        .get("rubric")
        .and_then(Value::as_str)
        .ok_or("which rubric? pass its name")?;
    let rubric = rubric_named(Path::new(&project.root_path), name)?;
    let card_id = params.get("cardId").and_then(Value::as_str);
    ran(crate::deciding::gate(
        Some(&project.id),
        card_id,
        &gate,
        &rubric,
        state,
    ))
}

/// Refuses questions that are not an object of at most [`MOST_QUESTIONS`].
pub(crate) fn counted(questions: &Value) -> Result<(), String> {
    let asked = questions
        .as_object()
        .filter(|asked| !asked.is_empty())
        .ok_or("pass `questions`: an object of question id to { type, instructions }")?;
    if asked.len() > MOST_QUESTIONS {
        return Err(format!(
            "{} questions is too many: ask at most {MOST_QUESTIONS} at once",
            asked.len()
        ));
    }
    Ok(())
}

fn cut_warning() -> String {
    format!("the state was longer than {MOST_STATE} characters and was cut; the answers read only its start")
}

/// A decision as the agent reads it; a skip is the agent's error.
pub(crate) fn answered(decided: Decided) -> Result<Value, String> {
    match decided {
        Decided::Answered {
            answers,
            cost_usd,
            latency_ms,
            truncated,
        } => {
            let mut said = json!({
                "answers": crate::deciding::readable(&answers),
                "cost_usd": cost_usd,
                "latency_ms": latency_ms,
            });
            if truncated {
                said["warning"] = json!(cut_warning());
            }
            Ok(said)
        }
        Decided::Skipped(skip) => Err(skip.reason()),
    }
}

/// A rubric's run as the agent reads it.
pub(crate) fn ran(result: GateResult) -> Result<Value, String> {
    if let Some(skip) = result.skipped {
        return Err(skip.reason());
    }
    let mut said = json!({
        "outcome": result.outcome,
        "verdicts": result.verdicts,
        "answers": result.answers,
        "cost_usd": result.cost_usd,
        "latency_ms": result.latency_ms,
    });
    if result.truncated {
        said["warning"] = json!(cut_warning());
    }
    Ok(said)
}

/// A rubric's name: a file name of its own, with nothing that reaches out of
/// the rubrics folder.
pub(crate) fn rubric_name(name: &str) -> bool {
    (1..=64).contains(&name.len())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `<root>/.devpit/rubrics/<name>.json`, or the built-in rubric of that name.
pub(crate) fn rubric_named(root: &Path, name: &str) -> Result<Rubric, String> {
    let text = match rubric_text_in(root, ".devpit/rubrics", name)? {
        Some(text) => text,
        None => BUILT_IN
            .iter()
            .find(|(built, _)| *built == name)
            .map(|(_, text)| (*text).to_owned())
            .ok_or_else(|| {
                format!("no rubric `{name}`: none in .devpit/rubrics/ and none built in")
            })?,
    };
    rubric_of(name, &text)
}

/// The text of `<root>/<folder>/<name>.json`, read only from inside the
/// project; `None` when there is no such file.
pub(crate) fn rubric_text_in(
    root: &Path,
    folder: &str,
    name: &str,
) -> Result<Option<String>, String> {
    if !rubric_name(name) {
        return Err("a rubric's name is letters, digits, - and _".to_owned());
    }
    let Ok(real) = std::fs::canonicalize(root.join(folder).join(format!("{name}.json"))) else {
        return Ok(None);
    };
    let inside = std::fs::canonicalize(root)
        .map(|root| real.starts_with(root))
        .unwrap_or(false);
    if !inside {
        return Err(format!("the rubric `{name}` leads outside the project"));
    }
    let size = std::fs::metadata(&real)
        .map_err(|err| err.to_string())?
        .len();
    if size > MOST_RUBRIC_BYTES {
        return Err(format!(
            "the rubric `{name}` is larger than any rubric needs"
        ));
    }
    std::fs::read_to_string(&real)
        .map(Some)
        .map_err(|err| err.to_string())
}

/// A rubric's text read, under the name it was asked by.
pub(crate) fn rubric_of(name: &str, text: &str) -> Result<Rubric, String> {
    let mut rubric: Rubric = serde_json::from_str(text)
        .map_err(|err| format!("the rubric `{name}` is not one devpit reads: {err}"))?;
    counted(&rubric.questions)?;
    rubric.name = name.to_owned();
    Ok(rubric)
}

#[cfg(test)]
#[path = "agent_decide_tests.rs"]
mod tests;
