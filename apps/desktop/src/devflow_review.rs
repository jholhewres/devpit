//! `devflow_review`: a card's change reviewed by devflow.yaml's reviewers,
//! headless on the person's own CLIs, kept in the card's run, attached, and
//! signed on the card — and a Devflow review's findings in the card's diff,
//! where the person can call one a false positive.
//!
//! The rules are `devpit_devflow::review`'s and `findings`'s; this adds the
//! board, git, the CLIs on this machine and the window.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use devpit_core::Store;
use devpit_devflow::config::Agent;
use devpit_devflow::evidence::Card;
use devpit_devflow::review::{self, Git, Given, Reach};
use devpit_devflow::{runs, Config};
use devpit_rpc::{CardReview, Finding, Found, Project, Review, Severity};
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter};

use crate::devflow_evidence::{checkout, SIGNED};
use crate::steps::context::{context_of, Checkout};

/// What the agent's wait keeps for attaching and answering once the reviewers stop.
const SPARE: Duration = Duration::from_secs(30);
/// How a Devflow review's run is named among a card's reviews.
const RUN: &str = "devflow:";

pub(crate) fn respond(
    app: Option<&AppHandle>,
    project: &Project,
    config: &Config,
    cwd: &Path,
    params: &Value,
) -> Result<Value, String> {
    respond_with(app, project, config, cwd, params, &reach)
}

/// The CLI on this machine that runs `agent`, or why there is none.
fn reach(agent: Agent) -> Result<String, String> {
    let cli = devpit_agentcli::reviewing::Cli::of(agent.word())
        .ok_or("devpit does not run this CLI headless")?;
    let program = cli.program();
    if !crate::shell_launch::shell_knows().contains(program) {
        return Err("not installed".to_owned());
    }
    if agent == Agent::Claude && crate::devflow::claude_signed_in() == Some(false) {
        return Err("not signed in".to_owned());
    }
    Ok(program.to_owned())
}

/// A card of this project's board, by id.
fn card_here(store: &Store, project: &Project, id: &str) -> Result<devpit_core::CardRow, String> {
    let on_board = store
        .project_id_of_card(id)
        .map_err(|err| err.to_string())?;
    store
        .card(id)
        .map_err(|err| err.to_string())?
        .filter(|_| on_board.as_deref() == Some(project.id.as_str()))
        .ok_or_else(|| format!("no card `{id}` on this project's board"))
}

/// BASE, HEAD and the diff between them, in the checkout.
fn pinned(
    root: &Path,
    base: Option<&str>,
) -> Result<(String, String, Vec<String>, String), String> {
    let said = |err: devpit_git::GitError| err.to_string();
    let base = devpit_git::review_base(root, base).map_err(said)?;
    let head = devpit_git::head_of(root).map_err(said)?;
    let files = devpit_git::changed_since(root, &base).map_err(said)?;
    let diff = devpit_git::diff_since(root, &base).map_err(said)?;
    Ok((base, head, files, diff))
}

/// The brief of the epic the card was planned under.
fn brief_of(store: &Store, project: &Project, card: &str) -> Option<devpit_devflow::brief::Brief> {
    let edges = store.card_edges(&project.id).ok()?;
    let (_, epic) = edges.parents.iter().find(|(child, _)| child == card)?;
    devpit_devflow::brief::read(&store.card(epic).ok()??.body)
}

pub(crate) fn respond_with(
    app: Option<&AppHandle>,
    project: &Project,
    config: &Config,
    cwd: &Path,
    params: &Value,
    reach: Reach,
) -> Result<Value, String> {
    let asked = review::asked(params)?;
    let store = crate::projects::store().map_err(|err| err.message)?;
    let card = card_here(&store, project, &asked.card)?;
    let root = checkout(project, cwd);
    let pinned = match asked.run {
        None => Some(pinned(
            &root,
            asked.base.as_deref().or(card.base_ref.as_deref()),
        )?),
        Some(_) => None,
    };
    let branch = devpit_git::branch_at(&root).unwrap_or_default();
    let env = context_of(
        &card,
        Some(Checkout {
            path: &root,
            branch: &branch,
        }),
        Some(&project.root_path),
    )
    .for_a_command()
    .environment();
    let secrets = crate::kept_out::what_devpit_gave(&store);
    let redact = |text: &str| crate::kept_out::kept_out(text, &secrets);
    let evidence = crate::devflow_gate::newest(&root, &card.id, "evidence.json");
    let brief = brief_of(&store, project, &card.id);
    // Asked only where it can answer: else the review falls back as without it.
    let jev = crate::devflow_jev::Jev::of(project, config, &root, Some(&card.id), &[])
        .filter(crate::devflow_jev::Jev::answers);
    let judging = jev
        .as_ref()
        .map(|jev| move |rubric: &str, state: &Value| jev.judge(rubric, state));
    // Personas follow the layers as skills do, read from the project's root.
    let teams = crate::devflow::layered(Path::new(&project.root_path))
        .map(|layered| layered.applied)
        .unwrap_or_default();
    let personal = devpit_core::Store::root()
        .map(|home| devpit_core::home::devflow_reviewers(&home))
        .ok();
    let done = review::reviewed(&Given {
        root: &root,
        config,
        asked: &asked,
        card: Card {
            id: &card.id,
            title: &card.title,
            body: &card.body,
        },
        git: pinned.as_ref().map(|(base, head, files, diff)| Git {
            base,
            head,
            files,
            diff,
        }),
        brief: brief.as_ref(),
        evidence: evidence.as_ref().map(|(_, evidence)| evidence),
        env: &env,
        at: SystemTime::now(),
        budget: devpit_agentapi::client::wait_for("devflow_review").saturating_sub(SPARE),
        reach,
        redact: &redact,
        jev: judging.as_ref().map(|judge| judge as review::Judge),
        personas: review::Personas {
            personal: personal.as_deref(),
            teams: &teams,
        },
    })?;

    let run = root.join(&done.folder);
    let artifacts = crate::projects::project_home(&project.id)
        .map_err(|err| err.message)?
        .artifacts();
    let under = format!("{}/{}", runs::attached_under(&card.id), done.stamp);
    let attached = runs::attach(&run, &done.files(), &artifacts, &under);
    let links: Vec<(String, String)> = attached
        .names
        .iter()
        .map(|name| {
            let short = name.strip_prefix(&format!("{under}/")).unwrap_or(name);
            (short.to_owned(), artifacts.join(name).display().to_string())
        })
        .collect();
    let said = review::comment(&done, &links);
    let sayable = crate::cards::sayable(&said).map_err(|err| err.message)?;
    store
        .add_comment(&card.id, SIGNED, sayable)
        .map_err(|err| err.to_string())?;
    if let Some(app) = app {
        let _ = app.emit("board:changed", &project.id);
    }
    let mut answer = review::summary(&done);
    answer["attached"] =
        json!({ "under": under, "files": attached.names.len(), "left_out": attached.left_out });
    Ok(answer)
}

/// The newest Devflow review of a card, with its run's name.
fn newest_review(store: &Store, card_id: &str) -> Option<(PathBuf, String, Value)> {
    let root = crate::devflow_jev::checkout_of(store, card_id)?;
    let (run, findings) = crate::devflow_gate::newest(&root, card_id, review::FINDINGS)?;
    Some((root, run, findings))
}

/// A Devflow severity as the card's diff draws it.
fn drawn(severity: &str) -> Severity {
    match severity {
        "critical" | "high" => Severity::Blocking,
        "medium" => Severity::Worth,
        _ => Severity::Noted,
    }
}

/// One finding as the card's diff reads it: where, how bad, and why.
pub(crate) fn as_drawn(finding: &Value) -> Finding {
    let text = |key: &str| finding[key].as_str().unwrap_or_default();
    let mut why = format!(
        "[{}] {} — {}",
        text("axis"),
        text("title"),
        text("description")
    );
    if let Some(agreement) = finding["agreement"].as_str() {
        why.push_str(&format!(" ({agreement})"));
    }
    if let Some(acted) = finding["jev"]["acted"].as_array() {
        let acted: Vec<&str> = acted.iter().filter_map(Value::as_str).collect();
        why.push_str(&format!(" — Jev: {}", acted.join("; ")));
    }
    Finding {
        file: text("file").to_owned(),
        line: finding["line"]
            .as_u64()
            .and_then(|line| u32::try_from(line).ok()),
        severity: drawn(text("severity")),
        why,
    }
}

/// The card's newest Devflow review, as one more review its diff draws;
/// what the person called a false positive is what it shows set aside.
pub(crate) fn card_review(store: &Store, card_id: &str) -> Option<CardReview> {
    let (root, run, kept) = newest_review(store, card_id)?;
    let list = kept["findings"].as_array()?;
    if list.is_empty() {
        return None;
    }
    let review = Review {
        findings: list.iter().map(as_drawn).collect(),
        at_revision: kept["head"].as_str().map(str::to_owned),
        rubric: Some("devflow-review".to_owned()),
    };
    let now = devpit_git::head_of(&root).ok();
    let dismissed = list
        .iter()
        .enumerate()
        .filter(|(_, finding)| !devpit_devflow::findings::standing(finding))
        .filter_map(|(at, _)| u32::try_from(at).ok())
        .collect();
    Some(CardReview {
        run_id: format!("{RUN}{card_id}/{run}"),
        found: Found {
            standing: devpit_rpc::standing(&review, now.as_deref()),
            at_revision: review.at_revision.clone(),
            now,
            rubric: review.rubric.clone(),
            findings: review.findings,
        },
        dismissed,
    })
}

/// A review run as the card's diff names a Devflow one: its card and run.
pub(crate) fn devflow_run(run_id: &str) -> Option<(&str, &str)> {
    let (card, run) = run_id.strip_prefix(RUN)?.split_once('/')?;
    runs::plain_card(card).then_some((card, run))
}

/// The person's "not a problem" on one finding of a Devflow review, or
/// taking it back: kept on the finding in findings.json, which the gate
/// then no longer blocks on, and kept as a label on what Jev said of it.
/// Bringing back one Jev set aside overrules Jev.
pub(crate) fn mark(
    store: &Store,
    run_id: &str,
    at: u32,
    false_positive: bool,
) -> Result<(), String> {
    let (card_id, run) = devflow_run(run_id).ok_or("not a Devflow review")?;
    let root = crate::devflow_jev::checkout_of(store, card_id).ok_or("the card has no checkout")?;
    let (relative, path) = runs::file_in(&root, card_id, Some(run), review::FINDINGS)?;
    let mut kept: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .ok_or_else(|| format!("{relative} is not there"))?;
    let finding = kept["findings"]
        .get_mut(at as usize)
        .and_then(Value::as_object_mut)
        .ok_or("no such finding")?;
    if false_positive {
        finding.insert("false_positive".to_owned(), json!(true));
    } else {
        finding.remove("false_positive");
        if let Some(jev) = finding.get_mut("jev").and_then(Value::as_object_mut) {
            if jev.get("set_aside") == Some(&json!(true)) {
                jev.insert("set_aside".to_owned(), json!(false));
                jev.insert("overruled".to_owned(), json!(true));
            }
        }
    }
    crate::devflow_jev::finding_marked(store, &Value::Object(finding.clone()), false_positive);
    let config = crate::devflow::read(&root);
    if let devpit_devflow::State::Valid(_, config) = config {
        let list = kept["findings"].as_array().cloned().unwrap_or_default();
        kept["verdict"] = json!(devpit_devflow::findings::verdict(
            &list,
            &config.review.block_on
        ));
    }
    let text = serde_json::to_string_pretty(&kept).unwrap_or_default();
    std::fs::write(&path, text).map_err(|err| format!("{relative}: {err}"))
}

#[cfg(test)]
#[path = "devflow_review_tests.rs"]
mod tests;
