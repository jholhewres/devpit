//! The agent step: one headless turn, with a schema and a ceiling.

use devpit_agentcli as agent;
use devpit_core::Store;
use devpit_rpc::Step;
use serde::Deserialize;

use super::context::{injected, Context};
use super::Finished;

/// What an `agent` step needs to know, out of `step.config`.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct AgentConfig {
    /// The agent to run, by the name in its frontmatter.
    ///
    /// A subagent, not a CLI account — `profile` is that, and the two live in
    /// this JSON side by side without being related.
    agent: Option<String>,
    /// Which profile runs it: the program, its arguments and the environment
    /// that picks the account. Absent runs whatever the build's default is,
    /// which is what every step did before profiles existed.
    profile: Option<String>,
    /// What to ask. The card's title and body are appended to it.
    prompt: String,
    /// A JSON Schema the answer has to satisfy.
    #[serde(alias = "expects")]
    schema: Option<String>,
    /// The ceiling. A step without one does not run: an agent with no ceiling
    /// is a bill nobody agreed to.
    #[serde(alias = "capUsd")]
    budget_usd: Option<f64>,
    model: Option<String>,
    /// Skills this step allows. Only these; a skill the step never named does
    /// not reach the command.
    skills: Vec<String>,
    /// Context to put in front of the agent, by key. Reaches the process as
    /// environment variables and never as text pasted into a command.
    inject: Vec<String>,
    /// The field of the answer that carries the verdict, and the value that
    /// means "not yet". Absent means this step never sends a card back.
    verdict_field: Option<String>,
    sends_back_when: Option<String>,
}

pub fn run(
    store: &Store,
    card_id: &str,
    step: &Step,
    run_id: &str,
    session_id: &str,
    mut on_progress: impl FnMut(&str),
    on_start: impl FnMut(u32),
) -> Result<Finished, String> {
    let config: AgentConfig = serde_json::from_str(&step.config)
        .map_err(|err| format!("this step's config is not readable: {err}"))?;

    // No ceiling, no run. Said here rather than after the money is spent.
    let Some(cap) = config.budget_usd else {
        return Err("this step declares no spending cap, so it does not run".to_owned());
    };

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
    let catalogue = agent::read_every_agent(&agent::seed_sources());
    let named = config
        .agent
        .as_ref()
        .and_then(|name| catalogue.agents.iter().find(|a| &a.name == name).cloned());
    if let (Some(name), None) = (&config.agent, &named) {
        return Err(format!("no agent named `{name}` on this machine"));
    }
    let argument = named
        .as_ref()
        .map(|a| agent::as_argument(std::slice::from_ref(a)));

    // The card's checkout when this step wants one, the project otherwise.
    let cwd = crate::checkout::cwd_for(store, card_id, step, &mut on_progress)?;
    // On the run's row, so continuing this session later finds its folder.
    store
        .set_run_cwd(run_id, &cwd.display().to_string())
        .map_err(|err| err.to_string())?;
    let context = injected(
        &Context {
            card: card.id.clone(),
            card_title: card.title.clone(),
            card_body: card.body.clone(),
            branch: card.base_ref.clone(),
            worktree_path: Some(cwd.display().to_string()),
            project_path: store.project_of_card(card_id).ok().flatten(),
        },
        &config.inject,
    );

    // The hooks reach us through a settings file written next to the state,
    // so a turn tells the board what it is doing while it does it.
    let settings = super::hook_settings();

    // Which account this spends through. A named profile that has gone is an
    // error rather than a silent fall back to the default: the difference
    // between them is somebody's bill.
    let runner = match &config.profile {
        Some(id) => Some(crate::agent_profiles::runner_for(store, id)?),
        None => None,
    };

    let outcome = agent::run_turn_cancellable(
        &agent::Turn {
            prompt: &prompt,
            cwd: &cwd,
            agents: argument.as_deref(),
            schema: config.schema.as_deref(),
            budget_usd: Some(cap),
            model: config.model.as_deref(),
            settings: settings.as_deref(),
            session_id: Some(session_id),
            env: &context,
            runner: runner.as_ref(),
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

    // The CLI's own word for the session wins: its transcript is named by it.
    let said = outcome.session_id.as_deref().unwrap_or(session_id);
    if said != session_id {
        eprintln!("run {run_id} asked for session {session_id} and ran in {said}");
        store
            .set_run_session(run_id, said)
            .map_err(|err| err.to_string())?;
    }

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
#[path = "agent_tests.rs"]
mod tests;
