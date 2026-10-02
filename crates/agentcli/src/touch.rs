//! What a tool is about to touch, read from the input its hook carries.
//!
//! A closed list of fields, in a fixed order. A tool whose input has none of
//! them is shown by its name alone: guessing at a shape nobody recorded is how
//! a label starts lying the day the shape changes.

use devpit_rpc::{IslandAsked, Touch};
use serde_json::Value;

/// How long a target may run before it is cut, in characters.
const TARGET: usize = 60;
/// How much of an edit or a command the preview keeps, in characters.
const KEPT: usize = 4000;
/// How many questions, and choices to one, a question step keeps.
const QUESTIONS: usize = 4;
const CHOICES: usize = 8;
/// How long a question or a choice may run, in characters.
const ASKED: usize = 400;

/// What a step reads as after its tool's name: `invoice.ts`, `npm test`.
pub fn target_of(input: &Value) -> Option<String> {
    if let Some(first) = questions_in(input).first() {
        return Some(cut(first.question.trim(), TARGET));
    }
    if let Some(path) = path_in(input) {
        return Some(cut(base_name(path), TARGET));
    }
    if let Some(command) = text(input, "command") {
        return Some(cut(
            command.lines().next().unwrap_or_default().trim(),
            TARGET,
        ));
    }
    ["pattern", "query", "description"]
        .iter()
        .find_map(|field| text(input, field))
        .or_else(|| text(input, "url").map(without_scheme))
        .map(|said| cut(said.trim(), TARGET))
}

/// What the preview shows for a step, when the tool is one it can draw.
pub fn touch_of(tool: &str, input: &Value) -> Option<Touch> {
    let path = || path_in(input).map(str::to_owned);
    match tool {
        "Edit" => Some(Touch::Edit {
            path: path()?,
            before: cut(text(input, "old_string")?, KEPT),
            after: cut(text(input, "new_string").unwrap_or_default(), KEPT),
        }),
        "MultiEdit" => {
            let first = input.get("edits")?.as_array()?.first()?;
            Some(Touch::Edit {
                path: path()?,
                before: cut(text(first, "old_string")?, KEPT),
                after: cut(text(first, "new_string").unwrap_or_default(), KEPT),
            })
        }
        "Write" => Some(Touch::Write {
            path: path()?,
            after: cut(text(input, "content").unwrap_or_default(), KEPT),
        }),
        "Read" => Some(Touch::Read {
            path: path()?,
            offset: number(input, "offset"),
            limit: number(input, "limit"),
        }),
        "Bash" => Some(Touch::Run {
            command: cut(text(input, "command")?, KEPT),
        }),
        "AskUserQuestion" => {
            let questions = questions_in(input);
            (!questions.is_empty()).then_some(Touch::Ask { questions })
        }
        "ExitPlanMode" => Some(Touch::Plan {
            plan: cut(text(input, "plan")?, KEPT),
        }),
        _ => None,
    }
}

/// The questions an `AskUserQuestion` input carries, each with its choices.
fn questions_in(input: &Value) -> Vec<IslandAsked> {
    let Some(asked) = input.get("questions").and_then(Value::as_array) else {
        return Vec::new();
    };
    asked
        .iter()
        .take(QUESTIONS)
        .filter_map(|one| {
            Some(IslandAsked {
                header: text(one, "header").map(|header| cut(header, TARGET)),
                question: cut(text(one, "question")?, ASKED),
                options: one
                    .get("options")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .take(CHOICES)
                    .filter_map(|option| text(option, "label").or_else(|| option.as_str()))
                    .map(|label| cut(label, ASKED))
                    .collect(),
                multi: one
                    .get("multiSelect")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn path_in(input: &Value) -> Option<&str> {
    ["file_path", "notebook_path", "path"]
        .iter()
        .find_map(|field| text(input, field))
}

fn text<'a>(input: &'a Value, field: &str) -> Option<&'a str> {
    input
        .get(field)?
        .as_str()
        .filter(|said| !said.trim().is_empty())
}

fn number(input: &Value, field: &str) -> Option<u32> {
    input
        .get(field)?
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
}

fn base_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(path)
}

fn without_scheme(url: &str) -> &str {
    url.split_once("://").map_or(url, |(_, rest)| rest)
}

/// At most `most` characters, on a character boundary, marked when cut.
fn cut(said: &str, most: usize) -> String {
    match said.char_indices().nth(most) {
        Some((at, _)) => format!("{}…", &said[..at]),
        None => said.to_owned(),
    }
}

#[cfg(test)]
#[path = "touch_tests.rs"]
mod tests;
