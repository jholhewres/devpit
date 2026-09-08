//! The agent step: one headless turn, with a schema and a ceiling.

use std::path::PathBuf;

use quockpit_agentcli as agent;
use quockpit_core::Store;
use quockpit_rpc::Step;
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

pub fn run(store: &Store, card_id: &str, step: &Step) -> Result<Finished, String> {
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
        return Err(format!("no agent named `{name}` in ~/.quockpit/agents"));
    }
    let argument = named
        .as_ref()
        .map(|a| agent::as_argument(std::slice::from_ref(a)));

    let outcome = agent::run_turn(
        &agent::Turn {
            prompt: &prompt,
            cwd: &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            agents: argument.as_deref(),
            schema: config.schema.as_deref(),
            budget_usd: config.budget_usd,
            model: config.model.as_deref(),
        },
        |_| {},
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
