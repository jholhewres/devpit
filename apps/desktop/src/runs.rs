//! Starting a step, and recording what it did.
//!
//! A run is opened before the work begins and closed when it ends, so a card
//! never has work happening with nothing on screen to show for it. When the
//! process dies mid-run the row stays `running` and says so — which is honest,
//! and better than a row that silently reports success.
//!
//! Nothing here retries. A failed run leaves the card where it is with the
//! reason on it, and the next move is a person's.

use std::path::PathBuf;

use quockpit_agentcli as agent;
use quockpit_core::Store;
use quockpit_rpc::{ErrorCode, RpcError, Run, RunState, Step, StepKind};
use serde::Deserialize;
use tauri::State;

use crate::sessions::SessionState;

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
}

/// Runs a step against a card, and returns the run it opened.
///
/// Only `agent` steps run here today. The other two kinds are recorded as a
/// failed run naming what is missing rather than silently doing nothing: a
/// card that looks like it started work and did not is worse than one that
/// says the step is not implemented yet.
pub fn start(
    store: &Store,
    _state: State<'_, SessionState>,
    card_id: &str,
    step: &Step,
) -> Result<Run, RpcError> {
    let run_id = store.start_run(card_id, &step.id)?;

    let outcome = match step.kind {
        StepKind::Agent => run_agent(store, card_id, step),
        StepKind::Session => Err("session steps arrive with the target terminal".to_owned()),
        StepKind::Command => Err("command steps arrive with tests and deploys".to_owned()),
    };

    match outcome {
        Ok(finished) => {
            store.finish_run(
                &run_id,
                if finished.ok { "ok" } else { "failed" },
                Some(&finished.output),
                Some(finished.cost_usd),
                Some(finished.duration_ms),
                None,
            )?;
            Ok(Run {
                id: run_id,
                step_id: step.id.clone(),
                step_name: step.name.clone(),
                state: if finished.ok {
                    RunState::Ok
                } else {
                    RunState::Failed
                },
                output: Some(finished.output),
                exit_code: None,
                cost_usd: Some(finished.cost_usd),
                duration_ms: Some(finished.duration_ms as f64),
                started_at: 0.0,
            })
        }
        Err(reason) => {
            store.finish_run(&run_id, "failed", Some(&reason), None, None, None)?;
            Ok(Run {
                id: run_id,
                step_id: step.id.clone(),
                step_name: step.name.clone(),
                state: RunState::Failed,
                output: Some(reason),
                exit_code: None,
                cost_usd: None,
                duration_ms: None,
                started_at: 0.0,
            })
        }
    }
}

struct Finished {
    ok: bool,
    output: String,
    cost_usd: f64,
    duration_ms: i64,
}

fn run_agent(store: &Store, card_id: &str, step: &Step) -> Result<Finished, String> {
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
            });
        }
    }

    Ok(Finished {
        ok: !outcome.is_error,
        output: outcome.result,
        cost_usd: outcome.cost_usd,
        duration_ms: outcome.duration_ms,
    })
}

fn agents_dir() -> Result<PathBuf, RpcError> {
    Ok(Store::root()
        .map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?
        .join("agents"))
}

/// Copies the agents already installed on this machine into `~/.quockpit`.
///
/// Runs on every start and never overwrites, so an agent the person edited
/// stays theirs.
pub fn seed_agents() -> usize {
    let Ok(dir) = agents_dir() else {
        return 0;
    };
    agent::seed_sources()
        .iter()
        .filter_map(|source| agent::seed_agents(&dir, source).ok())
        .sum()
}
