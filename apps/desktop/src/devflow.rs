//! Devflow through the agent API: its state with what devpit detected, a
//! setup, a change of one field, what the MCP server offers the agent
//! standing in a project, and an active project's skills.
//!
//! The file and the rules are `devpit_devflow`'s; this adds the machine's
//! half — the agent CLIs and their sign-ins, the Decisions key — and the
//! person's click, which only the window has.

use std::path::Path;

use devpit_core::{preference, Store};
use devpit_devflow::config::Agent;
use devpit_devflow::detect::{self, Cli, Machine};
use devpit_devflow::folder::{self, State, FILE};
use devpit_devflow::layers::{self, Home, Layered};
use devpit_devflow::preset::{self, Preset};
use devpit_devflow::{proposal, setup, Ask, Proposal};
use devpit_rpc::{Credentials, Project};
use serde_json::{json, Map, Value};
use tauri::AppHandle;

use crate::devflow_asks::{self as asks, globally_on, made};

pub(crate) const METHODS: [&str; 14] = [
    "devflow_offer",
    "devflow_status",
    "devflow_setup",
    "devflow_set",
    "devflow_skills",
    "devflow_skill",
    "devflow_brief",
    "devflow_plan",
    "devflow_evidence",
    "devflow_review",
    "devflow_gate",
    "devflow_session",
    "devflow_stage",
    "devflow_autopilot",
];

/// The stage a method of a stage's own belongs to: it answers only where
/// that stage is on, as the MCP server lists it.
pub(crate) fn stage_of(method: &str) -> Option<&'static str> {
    match method {
        "devflow_brief" => Some("briefing"),
        "devflow_plan" => Some("plan"),
        "devflow_evidence" => Some("execution"),
        "devflow_review" => Some("review"),
        "devflow_gate" => Some("completion"),
        _ => None,
    }
}

/// What the MCP server lists for a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Offered {
    /// Devflow switched off in Settings: the list devpit had before it.
    Off,
    /// The three tools that set it up.
    Setup,
    /// Those, and the tools of the stages switched on.
    Active(Vec<&'static str>),
}

/// The offer, from the global switch, the file, and the person's click.
pub(crate) fn offered(globally: bool, state: &State, switched_on: bool) -> Offered {
    match state {
        _ if !globally => Offered::Off,
        State::Valid(_, config) if config.enabled && switched_on => {
            Offered::Active(config.stages_on())
        }
        _ => Offered::Setup,
    }
}

/// The word `devpit_devflow_status` says for `state`.
pub(crate) fn state_word(state: &State, active: bool) -> &'static str {
    match state {
        State::Missing => "no file",
        State::Invalid(..) => "invalid",
        State::Valid(..) if active => "active",
        State::Valid(..) => "disabled",
    }
}

/// One of Devflow's methods, asked by the agent standing in `cwd`.
pub(crate) fn respond(
    app: Option<&AppHandle>,
    method: &str,
    project: &Project,
    cwd: &Path,
    params: &Value,
) -> Result<Value, String> {
    let store = crate::projects::store().map_err(|err| err.message)?;
    let root = Path::new(&project.root_path);
    let state = looked(app, &store, project, root);
    match method {
        "devflow_offer" => Ok(offer(&store, project, root, &state)),
        "devflow_status" => Ok(status(&store, project, root, &state)),
        "devflow_setup" => setup_of(app, &store, project, root, &state, params),
        "devflow_set" => set_of(app, &store, project, root, &state, params),
        // Answered where the stage is off too: a hook there says nothing.
        "devflow_gate" => crate::devflow_gate::respond(app, &store, project, &state, cwd, params),
        "devflow_session" => crate::devflow_gate::session(&store, project, &state, cwd, params),
        "devflow_stage" | "devflow_autopilot" => {
            stages_on(&store, project, &state)?;
            let State::Valid(_, config) = &state else {
                return Err("devflow.yaml does not read: devpit_devflow_status says why".to_owned());
            };
            match method {
                "devflow_stage" => {
                    crate::devflow_stage::respond(app, &store, project, config, cwd, params)
                }
                _ => crate::devflow_autopilot::respond(app, &store, project, params),
            }
        }
        "devflow_skills" | "devflow_skill" => crate::devflow_skills::respond(
            method,
            root,
            &stages_on(&store, project, &state)?,
            params,
        ),
        _ => match (stage_of(method), &state) {
            (Some(stage), State::Valid(_, config))
                if stages_on(&store, project, &state)?.contains(&stage) =>
            {
                match method {
                    "devflow_evidence" => {
                        crate::devflow_evidence::respond(app, project, config, cwd, params)
                    }
                    "devflow_review" => {
                        crate::devflow_review::respond(app, project, config, cwd, params)
                    }
                    _ => crate::devflow_plan::respond(app, method, project, params),
                }
            }
            (Some(stage), _) => {
                stages_on(&store, project, &state)?;
                Err(format!(
                    "Devflow's {stage} stage is switched off in this project's devflow.yaml."
                ))
            }
            (None, _) => Err(format!("devpit does not answer `{method}`")),
        },
    }
}

/// Where this devpit keeps the layers a project's Devflow is read over.
pub(crate) fn home() -> Home {
    Home::under(&Store::root().unwrap_or_default())
}

/// The project's devflow.yaml read over its layers — the person's own and
/// the teams it extends at their approved commits — which is what the flow
/// runs on. The text stays the project's own file, which a change edits.
pub(crate) fn read(root: &Path) -> State {
    match folder::read(root) {
        State::Valid(text, _) => match layers::layered(root, &text, &home()) {
            Ok(layered) => State::Valid(text, Box::new(layered.config)),
            Err(invalid) => State::Invalid(Some(text), invalid),
        },
        other => other,
    }
}

/// The layers as they stand, when the project's file reads.
pub(crate) fn layered(root: &Path) -> Option<Layered> {
    match folder::read(root) {
        State::Valid(text, _) => layers::layered(root, &text, &home()).ok(),
        _ => None,
    }
}

/// A change refused when the file it leaves would not read over its layers:
/// the project may not weaken what a layer locked.
fn layered_as(root: &Path, proposal: &Proposal) -> Result<(), String> {
    layers::layered(root, &proposal.yaml, &home())
        .map(|_| ())
        .map_err(|invalid| format!("{invalid}"))
}

/// Whether Devflow is on in `project`: a card's blockers hold it back only there.
pub(crate) fn is_on(project_id: &str, root: &Path) -> bool {
    let Ok(store) = crate::projects::store() else {
        return false;
    };
    matches!(
        offered(
            globally_on(&store),
            &read(root),
            made(&store, project_id, Ask::Enable)
        ),
        Offered::Active(_)
    )
}

/// The stages switched on, when Devflow is on here: nowhere else are its
/// skills given, whatever the MCP server last listed.
pub(crate) fn stages_on(
    store: &Store,
    project: &Project,
    state: &State,
) -> Result<Vec<&'static str>, String> {
    let switched_on = made(store, &project.id, Ask::Enable);
    match offered(globally_on(store), state, switched_on) {
        Offered::Active(stages) => Ok(stages),
        _ => Err("Devflow is not on in this project: devpit_devflow_status says why.".to_owned()),
    }
}

/// The file's state, raising the person's ask when it says `enabled: true`
/// and nobody here has clicked for it.
pub(crate) fn looked(
    app: Option<&AppHandle>,
    store: &Store,
    project: &Project,
    root: &Path,
) -> State {
    let state = read(root);
    if let State::Valid(_, config) = &state {
        if config.enabled && globally_on(store) && !made(store, &project.id, Ask::Enable) {
            asks::raise(app, &project.id, &project.name, Ask::Enable, true);
        }
    }
    state
}

fn offer(store: &Store, project: &Project, root: &Path, state: &State) -> Value {
    let offer = offered(
        globally_on(store),
        state,
        made(store, &project.id, Ask::Enable),
    );
    let orchestrator = project.orchestrator.is_some();
    let (word, stages) = match offer {
        // An orchestrator's folder is its notes, not a codebase to set a flow up in.
        _ if orchestrator => ("off", Vec::new()),
        Offered::Off => ("off", Vec::new()),
        Offered::Setup => ("setup", Vec::new()),
        Offered::Active(stages) => ("active", stages),
    };
    json!({
        "offer": word,
        "stages": stages,
        "watch": root.join(FILE).display().to_string(),
        // What else the server lists: an orchestrator's tools, and Decisions'.
        "orchestrator": orchestrator,
        "decisions": crate::decisions::key().is_some(),
    })
}

fn status(store: &Store, project: &Project, root: &Path, state: &State) -> Value {
    let switched_on = made(store, &project.id, Ask::Enable);
    let globally = globally_on(store);
    let active = matches!(offered(globally, state, switched_on), Offered::Active(_));
    let detected = detect::detected(root, machine(store, &project.id));
    let waiting: Vec<&str> = asks::waiting_in(&project.id)
        .into_iter()
        .map(Ask::word)
        .collect();
    let mut said = json!({
        "state": state_word(state, active),
        "file": FILE,
        "yaml": Value::Null,
        "recommended_checks": preset::checks(&detected),
        "open": detect::open(&detected),
        "detected": detected,
        "waiting_for_click": waiting,
        "note": Value::Null,
    });
    // The layers it extends, each applied or waiting for the person.
    if let Some(layered) = layered(root) {
        said["teams"] = json!(layered.teams);
        said["stages_from"] = json!(layered.stages_from);
    }
    let note = match state {
        State::Missing => {
            Some("No devflow.yaml yet: devpit_devflow_setup previews one.".to_owned())
        }
        State::Invalid(text, invalid) => {
            said["yaml"] = json!(text);
            said["error"] = json!(invalid);
            Some(format!(
                "devflow.yaml is invalid, so the Devflow tools stay off: {invalid}."
            ))
        }
        State::Valid(text, config) => {
            said["yaml"] = json!(text);
            said["config"] = config.to_json();
            (config.enabled && !switched_on).then(|| {
                "devflow.yaml says enabled: true, but the person has not switched Devflow on \
                 here yet: devpit shows them a confirmation in the app."
                    .to_owned()
            })
        }
    };
    said["note"] = match (globally, note) {
        (false, _) => json!("Devflow is switched off for every project in devpit's Settings."),
        (true, note) => json!(note),
    };
    said
}

pub(crate) fn setup_of(
    app: Option<&AppHandle>,
    store: &Store,
    project: &Project,
    root: &Path,
    state: &State,
    params: &Value,
) -> Result<Value, String> {
    let default = asks::preset(store);
    let word = params
        .get("preset")
        .and_then(Value::as_str)
        .unwrap_or(default.word());
    let preset = Preset::of(word)
        .ok_or_else(|| format!("`{word}` is not a preset: recommended or custom"))?;
    let set: Map<String, Value> = match params.get("set") {
        None | Some(Value::Null) => Map::new(),
        Some(Value::Object(set)) => set.clone(),
        Some(_) => return Err("`set` is an object of dotted path to value".to_owned()),
    };
    let detected = detect::detected(root, machine(store, &project.id));
    let proposal =
        proposal::setup(state, preset, &set, &detected).map_err(|why| why.to_string())?;
    layered_as(root, &proposal)?;
    let apply = params.get("apply").and_then(Value::as_bool) == Some(true);
    let checks: Vec<Value> = proposal
        .config
        .stages
        .execution
        .checks
        .iter()
        .map(|check| {
            let script = proposal.scripts.iter().find(|one| one.name == check.name);
            json!({
                "name": check.name,
                "run": check.run,
                "command": script.map(|one| one.command.as_str()),
                "source": script.and_then(|one| one.source.as_deref()),
                "report": check.report,
            })
        })
        .collect();
    let files = if apply {
        setup::write(root, &proposal.files)?
    } else {
        setup::preview(root, &proposal.files)
    };
    let waiting = if apply {
        settled(app, store, project, &proposal, true)
    } else {
        Vec::new()
    };
    Ok(json!({
        "applied": apply,
        "preset": word,
        "yaml": proposal.yaml,
        "checks": checks,
        "files": files,
        "open": detect::open(&detected),
        "asks": proposal.asks,
        "waiting_for_click": waiting,
        "note": if apply {
            "Written. devpit never commits: these files are the person's to commit. Devflow is \
             still off until the person clicks the confirmation devpit now shows in the app."
        } else {
            "Nothing was written. apply: true writes these files, once the person said yes to \
             this preview."
        },
    }))
}

pub(crate) fn set_of(
    app: Option<&AppHandle>,
    store: &Store,
    project: &Project,
    root: &Path,
    state: &State,
    params: &Value,
) -> Result<Value, String> {
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .ok_or("pass the field as a dotted `path`, such as review.block_on")?;
    let value = params
        .get("value")
        .ok_or("pass the new `value` for the field")?;
    let proposal = proposal::set(state, path, value).map_err(|why| why.to_string())?;
    layered_as(root, &proposal)?;
    let apply = params.get("apply").and_then(Value::as_bool) == Some(true);
    let before = match state {
        State::Valid(_, config) => preset::value_at(&config.to_json(), path).clone(),
        _ => Value::Null,
    };
    let written = apply && !proposal.changes.is_empty();
    if written {
        setup::write(root, &proposal.files)?;
        if !proposal.config.enabled && made(store, &project.id, Ask::Enable) {
            // Switched off: switching on again is the person's click again.
            let _ = asks::remember(store, &project.id, Ask::Enable, false);
        }
        asks::rang(app, &project.id);
    }
    let waiting = if apply {
        settled(app, store, project, &proposal, false)
    } else {
        Vec::new()
    };
    let wrote = match (apply, written) {
        (true, true) => "Written. devpit never commits: the file is the person's to commit.",
        (true, false) => "Nothing was written to the file.",
        (false, _) => "Nothing was written. apply: true writes it, once the person said yes.",
    };
    let clicked = match (proposal.asks.is_empty(), apply) {
        (true, _) => "",
        (false, true) => {
            " Switching Devflow on and moving Decisions to enforce are the person's click: \
             devpit now shows them the confirmation in the app."
        }
        (false, false) => {
            " Switching Devflow on and moving Decisions to enforce are never written by a \
             tool: they wait for the person's click on a confirmation in the app."
        }
    };
    let note = format!("{wrote}{clicked}");
    Ok(json!({
        "applied": written,
        "path": path,
        "before": before,
        "after": preset::value_at(&proposal.config.to_json(), path),
        "changes": proposal.changes,
        "yaml": proposal.yaml,
        "laid_out": proposal.laid_out,
        "asks": proposal.asks,
        "waiting_for_click": waiting,
        "note": note,
    }))
}

/// Raises what a written proposal leaves to the person: what it asked for,
/// and, after a setup, switching Devflow on when it is not on here yet.
/// Answers what waits.
fn settled(
    app: Option<&AppHandle>,
    store: &Store,
    project: &Project,
    proposal: &Proposal,
    after_setup: bool,
) -> Vec<&'static str> {
    let mut wanted = proposal.asks.clone();
    let on = proposal.config.enabled && made(store, &project.id, Ask::Enable);
    if after_setup && !on && !wanted.contains(&Ask::Enable) {
        wanted.push(Ask::Enable);
    }
    for ask in &wanted {
        asks::raise(app, &project.id, &project.name, *ask, false);
    }
    asks::waiting_in(&project.id)
        .into_iter()
        .map(Ask::word)
        .collect()
}

/// The reviewer CLIs on this machine, and the Decisions key, as the app
/// already knows them.
pub(crate) fn machine(store: &Store, project_id: &str) -> Machine {
    let knows = crate::shell_launch::shell_knows();
    let clis = Agent::ALL
        .iter()
        .map(|agent| {
            let program = devpit_pty::agents::KNOWN
                .iter()
                .find(|known| known.id == agent.word())
                .and_then(|known| known.launch.split_whitespace().next())
                .unwrap_or(agent.word());
            let installed = knows.contains(program);
            Cli {
                name: *agent,
                installed,
                signed_in: (installed && *agent == Agent::Claude)
                    .then(claude_signed_in)
                    .flatten(),
            }
        })
        .collect();
    let opted_out: Vec<String> = store
        .preference(preference::DECISIONS_OPTED_OUT)
        .ok()
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    // How the repository already works: its branches and what its commits name.
    let root = store
        .project(project_id)
        .ok()
        .flatten()
        .map(|row| std::path::PathBuf::from(row.root_path));
    let branches = root
        .as_deref()
        .and_then(|root| devpit_git::branches(root).ok())
        .map(|all| all.into_iter().map(|one| one.name).collect())
        .unwrap_or_default();
    let subjects = root
        .as_deref()
        .and_then(|root| devpit_git::history(root, 200, 0).ok())
        .map(|all| all.into_iter().map(|one| one.subject).collect())
        .unwrap_or_default();
    Machine {
        clis,
        decisions_key: crate::decisions::key().is_some(),
        decisions_opted_out: opted_out.iter().any(|id| id == project_id),
        branches,
        subjects,
        installed_skills: installed_skills(),
    }
}

/// Where Claude Code keeps the skills already installed for it.
pub(crate) fn installed_folder() -> Option<std::path::PathBuf> {
    Some(devpit_agentcli::cli_config::config_dir()?.join("skills"))
}

/// The skills already installed for Claude Code, by name: an own stage can
/// use one where it is.
fn installed_skills() -> Vec<String> {
    let Some(folder) = installed_folder() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.path().join("SKILL.md").is_file())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .take(200)
        .collect()
}

/// The sign-in this process's Claude Code runs on, read as the footer reads
/// it. devpit has no such check for the other CLIs, so they say nothing.
pub(crate) fn claude_signed_in() -> Option<bool> {
    let directory = devpit_agentcli::cli_config::config_dir()?;
    match devpit_agentcli::cli_config::sign_in_at(&directory) {
        Credentials::Saved => Some(true),
        Credentials::Missing | Credentials::Expired => Some(false),
        Credentials::Unknown => None,
    }
}

#[cfg(test)]
#[path = "devflow_tests.rs"]
mod tests;
