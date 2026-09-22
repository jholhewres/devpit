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
        Err(why) => {
            eprintln!("devpit: {why}\n\n{}", guide::GUIDE);
            2
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Parsed {
    Guide,
    Ask(String, Value),
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
        Some("move") => {
            let id = one(&plain, "a card id")?;
            let column = plain.get(1).ok_or("move needs a column id")?;
            ask("move", json!({ "cardId": id, "columnId": column }))
        }
        Some(other) => Err(format!("`{other}` is not a devpit agent command")),
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
