//! The agent step: one headless turn, with a schema and a ceiling.

use std::path::PathBuf;

use devpit_agentcli as agent;
use devpit_core::Store;
use devpit_rpc::Step;
use serde::Deserialize;

use super::{agents_dir, Finished};

/// What an `agent` step needs to know, out of `step.config`.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct AgentConfig {
    /// The agent to run, by the name in its frontmatter.
    agent: Option<String>,
    /// What to ask. The card's title and body are appended to it.
    prompt: String,
    /// A JSON Schema the answer has to satisfy.
    schema: Option<String>,
    /// The ceiling. Absent means no ceiling, which is a choice the step makes
    /// explicitly rather than one it falls into.
    budget_usd: Option<f64>,
    model: Option<String>,
    /// The field of the answer that carries the verdict, and the value that
    /// means "not yet". Absent means this step never sends a card back.
    verdict_field: Option<String>,
    sends_back_when: Option<String>,
}

pub fn run(
    store: &Store,
    card_id: &str,
    step: &Step,
    mut on_progress: impl FnMut(&str),
    on_start: impl FnMut(u32),
) -> Result<Finished, String> {
    let config: AgentConfig = serde_json::from_str(&step.config)
        .map_err(|err| format!("this step's config is not readable: {err}"))?;

    let card = store
        .card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("no such card")?;

    let prompt = format!(
        "{}\n\n---\n\n# {}\n\n{}",
        config.prompt, card.title, card.body
    );

    // Only the agent this step names is passed. Handing the CLI the whole
    // catalogue would make every step's behaviour depend on files it never
    // mentions.
    let catalogue = agent::read_agents(&agents_dir().map_err(|err| err.to_string())?);
    let named = config
        .agent
        .as_ref()
        .and_then(|name| catalogue.agents.iter().find(|a| &a.name == name).cloned());
    if let (Some(name), None) = (&config.agent, &named) {
        return Err(format!("no agent named `{name}` in ~/.devpit/agents"));
    }
    let argument = named
        .as_ref()
        .map(|a| agent::as_argument(std::slice::from_ref(a)));

    // The hooks reach us through a settings file written next to the state,
    // so a turn tells the board what it is doing while it does it.
    let settings = super::hook_settings();

    let outcome = agent::run_turn_cancellable(
        &agent::Turn {
            prompt: &prompt,
            cwd: &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            agents: argument.as_deref(),
            schema: config.schema.as_deref(),
            budget_usd: config.budget_usd,
            model: config.model.as_deref(),
            settings: settings.as_deref(),
        },
        |line| {
            // Only the assistant's own words. The stream also carries hook
            // chatter and rate-limit notices, and a card relaying those reads
            // as the agent talking about the machinery rather than the work.
            if let Some(text) = assistant_text(line) {
                on_progress(&text);
            }
        },
        on_start,
    )
    .map_err(|err| err.to_string())?;

    // A schema the answer does not satisfy is a failure, and the card does not
    // advance. Prose where fields were asked for is the case this catches.
    if let Some(schema) = &config.schema {
        if let Err(reason) = agent::validates(&outcome.result, schema) {
            return Ok(Finished {
                ok: false,
                output: format!(
                    "the answer did not match the schema: {reason}\n\n{}",
                    outcome.result
                ),
                cost_usd: outcome.cost_usd,
                duration_ms: outcome.duration_ms,
                exit_code: None,
            });
        }
    }

    Ok(Finished {
        ok: !outcome.is_error,
        output: outcome.result,
        cost_usd: outcome.cost_usd,
        duration_ms: outcome.duration_ms,
        exit_code: None,
    })
}

/// The words an assistant fragment carries, if it carries any.
///
/// The stream interleaves hook events, init and rate-limit notices with the
/// answer. Relaying all of it would make a card show the machinery instead of
/// the work.
fn assistant_text(line: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("type")?.as_str()? != "assistant" {
        return None;
    }
    let blocks = value.get("message")?.get("content")?.as_array()?;
    let text: String = blocks
        .iter()
        .filter(|block| block.get("type").and_then(|t| t.as_str()) == Some("text"))
        .filter_map(|block| block.get("text").and_then(|t| t.as_str()))
        .collect::<Vec<_>>()
        .join("");
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_assistant_fragment_yields_its_words() {
        let line = r#"{"type":"assistant","message":{"content":[
            {"type":"text","text":"Looking at the parser"}]}}"#;
        assert_eq!(
            assistant_text(line).as_deref(),
            Some("Looking at the parser")
        );
    }

    /// Thinking blocks and tool calls are not words to show.
    #[test]
    fn a_fragment_with_no_text_yields_nothing() {
        let line = r#"{"type":"assistant","message":{"content":[
            {"type":"thinking","thinking":"..."}]}}"#;
        assert_eq!(assistant_text(line), None);
    }

    /// The machinery is not the work.
    #[test]
    fn the_other_lines_are_not_relayed() {
        for line in [
            r#"{"type":"system","subtype":"init"}"#,
            r#"{"type":"rate_limit_event"}"#,
            r#"{"type":"result","result":"done"}"#,
            "not json at all",
        ] {
            assert_eq!(assistant_text(line), None, "{line}");
        }
    }
}
