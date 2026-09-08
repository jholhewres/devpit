//! Starting a step, and recording what it did.
//!
//! A run is opened before the work begins and closed when it ends, so a card
//! never has work happening with nothing on screen to show for it. When the
//! process dies mid-run the row stays `running` and says so — which is honest,
//! and better than a row that silently reports success.
//!
//! **The work happens off the command's thread.** A turn takes seconds; doing
//! it inline would freeze the drag that started it for exactly as long, and a
//! board that locks up while it thinks is a board nobody drags things onto.
//! The command returns a `running` row and the thread finishes it, announcing
//! itself on `run:changed` so the card can catch up.
//!
//! Nothing here retries. A failed run leaves the card where it is with the
//! reason on it, and the next move is a person's.

use std::path::PathBuf;

use quockpit_agentcli as agent;
use quockpit_core::Store;
use quockpit_rpc::{ErrorCode, RpcError, Run, RunState, Step, StepKind};
use serde::Deserialize;
use tauri::{AppHandle, Emitter};

/// What a `session` step needs to know, out of `step.config`.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct SessionConfig {
    /// Give the session its own git worktree, named after the card.
    worktree: bool,
    model: Option<String>,
}

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

/// Whether an answer asked for the card to go back, and why.
///
/// The only automatic transition in the product. It exists because a review
/// that says "revise" and then leaves the card sitting in the reviewed column
/// is a review nobody acts on.
pub fn sends_back(config: &str, answer: &str) -> Option<String> {
    let config: AgentConfig = serde_json::from_str(config).ok()?;
    let field = config.verdict_field?;
    let back = config.sends_back_when?;
    let answer: serde_json::Value = serde_json::from_str(answer).ok()?;
    let verdict = answer.get(&field)?.as_str()?;
    (verdict == back).then(|| format!("{field}: {verdict}"))
}

/// Runs a step against a card, and returns the run it opened.
///
/// `command` steps are recorded as a failed run naming what is missing rather
/// than silently doing nothing: a card that looks like it started work and did
/// not is worse than one that says the step is not built yet.
///
/// `came_from` is where a verdict sends the card back to. Without it a review
/// that says "revise" leaves the card sitting in the reviewed column, which is
/// a review nobody acts on.
pub fn start(
    app: AppHandle,
    store: &Store,
    card_id: &str,
    step: &Step,
    came_from: Option<&str>,
) -> Result<Run, RpcError> {
    let run_id = store.start_run(card_id, &step.id)?;

    let card = card_id.to_owned();
    let id = run_id.clone();
    let back_to = came_from.map(ToOwned::to_owned);
    // Read before the move: the row the command returns describes the step
    // that is about to run, and the thread takes ownership of the step itself.
    let step_id = step.id.clone();
    let step_name = step.name.clone();
    let step = step.clone();
    std::thread::spawn(move || {
        // The thread opens its own connection: SQLite handles are not shared
        // across threads, and the row it has to close is already committed.
        let Ok(store) = Store::open_default() else {
            return;
        };

        let outcome = match step.kind {
            StepKind::Agent => run_agent(&store, &card, &step),
            StepKind::Session => start_session(&store, &card, &step),
            StepKind::Command => Err("command steps arrive with tests and deploys".to_owned()),
        };

        let answered = match &outcome {
            Ok(finished) if finished.ok => Some(finished.output.clone()),
            _ => None,
        };
        let closed = match outcome {
            Ok(finished) => store.finish_run(
                &id,
                if finished.ok { "ok" } else { "failed" },
                Some(&finished.output),
                Some(finished.cost_usd),
                Some(finished.duration_ms),
                None,
            ),
            Err(reason) => store.finish_run(&id, "failed", Some(&reason), None, None, None),
        };

        // A failure to record is worth saying out loud: the run finished and
        // the screen would otherwise show it running forever.
        if let Err(err) = closed {
            eprintln!("could not record the end of run {id}: {err}");
        }

        // The verdict is the only thing in the product that moves a card on
        // its own, and it only ever moves it backwards.
        if let (Some(column), Some(answer)) = (&back_to, &answered) {
            if let Some(why) = sends_back(&step.config, answer) {
                let position = store.cards_in_column(column).unwrap_or(0);
                let _ = store.move_card(&card, column, position);
                let _ = store.note_on_card(&card, &format!("sent back — {why}"));
            }
        }
        let _ = app.emit("run:changed", &card);
    });

    Ok(Run {
        id: run_id,
        step_id,
        step_name,
        state: RunState::Running,
        output: None,
        exit_code: None,
        cost_usd: None,
        duration_ms: None,
        started_at: 0.0,
    })
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

/// Starts a background session for this card and records the handle.
///
/// The session is not brought into a terminal here. It runs detached, and the
/// person attaches it to the target terminal when they want to sit in front of
/// it — which is the whole point of the target being one terminal rather than
/// a pane per card.
fn start_session(store: &Store, card_id: &str, step: &Step) -> Result<Finished, String> {
    let config: SessionConfig = serde_json::from_str(&step.config)
        .map_err(|err| format!("this step's config is not readable: {err}"))?;

    let card = store
        .card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("no such card")?;

    let project = store
        .project_of_card(card_id)
        .map_err(|err| err.to_string())?
        .ok_or("this card has no project on disk")?;
    let cwd = PathBuf::from(&project);

    // A stable id chosen here rather than discovered later: it is what names
    // the transcript, and the transcript is where a session the person drove
    // by hand reports what it spent.
    let session_id = uuid_like(card_id);
    let worktree = config.worktree.then(|| slug(&card.title));

    let short_id = agent::start_background(
        &cwd,
        Some(&session_id),
        worktree.as_deref(),
        config.model.as_deref(),
    )
    .map_err(|err| err.to_string())?;

    let transcript = std::env::var_os("HOME")
        .map(PathBuf::from)
        .map(|home| agent::transcript_path(&home, &cwd, &session_id));

    store
        .link_session(
            card_id,
            &short_id,
            &session_id,
            transcript.as_ref().and_then(|p| p.to_str()),
        )
        .map_err(|err| err.to_string())?;

    Ok(Finished {
        ok: true,
        output: format!("session {short_id} is running; attach it to the terminal to drive it"),
        cost_usd: 0.0,
        duration_ms: 0,
    })
}

/// A session id shaped like the UUID the CLI expects, derived from the card so
/// the same card keeps the same id across restarts.
fn uuid_like(card_id: &str) -> String {
    let digest: Vec<String> = card_id
        .bytes()
        .cycle()
        .take(16)
        .map(|b| format!("{b:02x}"))
        .collect();
    let hex = digest.concat();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

/// A branch-safe name from a card title.
fn slug(title: &str) -> String {
    let mut out = String::new();
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').chars().take(40).collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    const REVIEW: &str =
        r#"{"prompt":"review it","verdictField":"verdict","sendsBackWhen":"revise"}"#;

    /// The only automatic transition in the product, and it only goes back.
    #[test]
    fn a_verdict_of_revise_sends_the_card_back() {
        let why = sends_back(REVIEW, r#"{"verdict":"revise","findings":["no tests"]}"#)
            .expect("should send back");
        assert!(why.contains("revise"), "{why}");
    }

    #[test]
    fn an_approving_verdict_leaves_the_card_where_it_is() {
        assert_eq!(sends_back(REVIEW, r#"{"verdict":"approved"}"#), None);
    }

    /// A step that declares no verdict never moves a card on its own.
    #[test]
    fn a_step_without_a_verdict_never_sends_anything_back() {
        assert_eq!(
            sends_back(r#"{"prompt":"refine it"}"#, r#"{"verdict":"revise"}"#),
            None
        );
    }

    /// Prose where a verdict was expected is not a verdict. Reading one out of
    /// it would move cards on a guess.
    #[test]
    fn an_answer_that_is_not_json_moves_nothing() {
        assert_eq!(sends_back(REVIEW, "I think you should revise this"), None);
    }

    /// A branch name has to survive a title with punctuation in it.
    #[test]
    fn a_card_title_becomes_a_branch_safe_name() {
        assert_eq!(slug("Fix the OAuth flow!"), "fix-the-oauth-flow");
        assert_eq!(slug("  spaces  everywhere  "), "spaces-everywhere");
        assert!(slug(&"x".repeat(80)).len() <= 40);
    }

    /// The same card keeps the same session id across restarts, which is what
    /// makes its transcript findable later.
    #[test]
    fn a_card_always_gets_the_same_session_id() {
        let first = uuid_like("card_01HX");
        assert_eq!(first, uuid_like("card_01HX"));
        assert_ne!(first, uuid_like("card_01HY"));
        assert_eq!(first.len(), 36, "{first} is not shaped like a uuid");
        assert_eq!(first.matches('-').count(), 4);
    }
}
