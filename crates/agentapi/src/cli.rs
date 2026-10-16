//! `devpit agent …`: the board from a shell.

use std::path::Path;

use serde_json::{json, Map, Value};

use crate::{client, guide};

/// Runs one `devpit agent` command and answers with the exit code.
pub fn run(root: &Path, args: &[String]) -> i32 {
    let cwd = crate::standing();
    match parsed(args) {
        Ok(Parsed::Guide) => {
            print!("{}", guide::GUIDE);
            0
        }
        // A report from any agent: never fails the agent that sent it.
        Ok(Parsed::Hook(call)) => {
            let body = crate::events::payload(&call, &stdin_json(), &cwd.display().to_string());
            let pane = std::env::var(crate::hook::PANE_ENV).unwrap_or_default();
            let _ = crate::hook::post(
                root,
                body.as_bytes(),
                &pane,
                None,
                std::time::Duration::from_millis(1500),
            );
            0
        }
        // Devflow's session-start hook: always 0, and silent when the app
        // has nothing to say, so a session never fails to start on it.
        Ok(Parsed::Context(agent)) => {
            let payload = stdin_json();
            let pane = std::env::var(crate::hook::PANE_ENV).unwrap_or_default();
            let params = crate::devflow::context_params(&payload, &pane, &agent);
            let here = payload
                .get("cwd")
                .and_then(Value::as_str)
                .map_or(cwd, std::path::PathBuf::from);
            if let Some(said) = client::ask(root, "devflow_session", params, &here)
                .ok()
                .and_then(|answer| crate::devflow::context_reply(&answer))
            {
                println!("{said}");
            }
            0
        }
        // A skill is read as the markdown it is.
        Ok(Parsed::Skill(name)) => {
            match client::ask(root, "devflow_skill", json!({ "name": name }), &cwd) {
                Ok(answer) => {
                    print!("{}", answer["text"].as_str().unwrap_or_default());
                    0
                }
                Err(why) => {
                    eprintln!("devpit: {why}");
                    1
                }
            }
        }
        Ok(Parsed::Ask(method, params)) => match client::ask(root, &method, params, &cwd) {
            Ok(answer) => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&answer).unwrap_or_default()
                );
                0
            }
            Err(why) => {
                eprintln!("devpit: {why}");
                1
            }
        },
        // The error and where to read more: the whole guide buried it.
        Err(why) => {
            eprintln!("devpit: {why}\n{PARSE_HINT}");
            2
        }
    }
}

/// Said under a command that did not parse.
pub(crate) const PARSE_HINT: &str = "`devpit-agent guide` says what it takes";

#[derive(Debug, PartialEq)]
pub(crate) enum Parsed {
    Guide,
    Ask(String, Value),
    /// `devpit agent hook --agent NAME EVENT …`: an event from an agent that
    /// is not Claude Code, in the shape devpit reads.
    Hook(crate::events::Call),
    /// `devpit agent devflow-context [--agent NAME]`: what Devflow's
    /// session-start hook adds, for that agent CLI.
    Context(String),
    /// `devpit agent devflow <stage>`: the stage's skill, by name.
    Skill(String),
}

/// The JSON an agent piped in, when it piped any: an object, or nothing.
fn stdin_json() -> Value {
    use std::io::{IsTerminal, Read};
    let stdin = std::io::stdin();
    if stdin.is_terminal() {
        return Value::Null;
    }
    let mut text = String::new();
    let _ = stdin
        .lock()
        .take(crate::events::MOST_BYTES)
        .read_to_string(&mut text);
    serde_json::from_str::<Value>(&text)
        .ok()
        .filter(Value::is_object)
        .unwrap_or(Value::Null)
}

/// The method and its parameters, from the words after `devpit agent`.
pub(crate) fn parsed(args: &[String]) -> Result<Parsed, String> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    let (options, plain) = split(&words[1.min(words.len())..]);
    let ask = |method: &str, params: Value| Ok(Parsed::Ask(method.to_owned(), params));
    match words.first().copied() {
        None | Some("guide" | "help" | "--help" | "-h") => Ok(Parsed::Guide),
        Some("context") => ask("context", json!({})),
        Some("board") => ask("board", json!({})),
        Some("methods") => ask("methods", json!({})),
        Some("card") => ask("card", json!({ "cardId": one(&plain, "a card id")? })),
        Some("comment") => {
            let id = one(&plain, "a card id")?;
            let text = plain[1..].join(" ");
            if text.trim().is_empty() {
                return Err("comment needs something to say".to_owned());
            }
            ask("comment", json!({ "cardId": id, "body": text }))
        }
        Some("comment-edit") => {
            let id = one(&plain, "a comment id")?;
            let text = plain[1..].join(" ");
            if text.trim().is_empty() {
                return Err("comment-edit needs the new text".to_owned());
            }
            ask("comment_edit", json!({ "commentId": id, "body": text }))
        }
        Some("comment-delete") => ask(
            "comment_delete",
            json!({ "commentId": one(&plain, "a comment id")? }),
        ),
        Some("create") => {
            let title = plain.join(" ");
            if title.trim().is_empty() {
                return Err("create needs a title".to_owned());
            }
            let mut params = Map::new();
            params.insert("title".into(), json!(title));
            copy(
                &options,
                &mut params,
                &[("body", "body"), ("column", "columnId")],
            );
            ask("create", Value::Object(params))
        }
        Some("update") => {
            let mut params = Map::new();
            params.insert("cardId".into(), json!(one(&plain, "a card id")?));
            copy(
                &options,
                &mut params,
                &[("title", "title"), ("body", "body")],
            );
            ask("update", Value::Object(params))
        }
        Some("hook") => crate::events::parsed(&options, &plain).map(Parsed::Hook),
        Some("devflow-context") => Ok(Parsed::Context(
            options
                .iter()
                .find(|(name, _)| *name == "agent")
                .map_or("claude", |(_, agent)| agent)
                .to_owned(),
        )),
        Some("devflow") => devflow(&options, &plain),
        Some("move") => {
            let id = one(&plain, "a card id")?;
            let column = plain.get(1).ok_or("move needs a column id")?;
            ask("move", json!({ "cardId": id, "columnId": column }))
        }
        Some(other) => Err(format!("`{other}` is not a devpit-agent command")),
    }
}

/// `devpit agent devflow <stage>`: the stage's skill, or with `--json` its
/// tool asked with that object, as the MCP tool would be.
fn devflow(options: &[(&str, &str)], plain: &[&str]) -> Result<Parsed, String> {
    let stage = one(plain, "a stage")?;
    let stages = crate::devflow::STAGE_METHODS;
    let (_, method) = stages
        .iter()
        .find(|(name, _)| *name == stage)
        .ok_or_else(|| {
            let names: Vec<&str> = stages.iter().map(|(name, _)| *name).collect();
            format!("`{stage}` is not a Devflow stage: {}", names.join(", "))
        })?;
    match options {
        [] => Ok(Parsed::Skill(format!("devflow-{stage}"))),
        [("json", said)] => {
            let params: Value = serde_json::from_str(said)
                .ok()
                .filter(Value::is_object)
                .ok_or("--json takes one JSON object, the tool's arguments")?;
            Ok(Parsed::Ask((*method).to_owned(), params))
        }
        _ => Err("devflow takes a stage, and --json with the tool's arguments".to_owned()),
    }
}

/// `--name value` pairs apart from the plain words.
fn split<'a>(words: &[&'a str]) -> (Vec<(&'a str, &'a str)>, Vec<&'a str>) {
    let mut options = Vec::new();
    let mut plain = Vec::new();
    let mut at = 0;
    while at < words.len() {
        if let Some(name) = words[at].strip_prefix("--") {
            options.push((name, words.get(at + 1).copied().unwrap_or("")));
            at += 2;
        } else {
            plain.push(words[at]);
            at += 1;
        }
    }
    (options, plain)
}

fn one<'a>(plain: &[&'a str], what: &str) -> Result<&'a str, String> {
    plain
        .first()
        .copied()
        .ok_or_else(|| format!("that needs {what}"))
}

fn copy(options: &[(&str, &str)], into: &mut Map<String, Value>, names: &[(&str, &str)]) {
    for (flag, key) in names {
        if let Some((_, value)) = options.iter().find(|(name, _)| name == flag) {
            into.insert((*key).to_owned(), json!(value));
        }
    }
}

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
