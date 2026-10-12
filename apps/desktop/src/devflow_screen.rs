//! Devflow's screen: a project's devflow.yaml as the window draws it, and the
//! person's edits to it. Every answer goes through what the MCP tools call —
//! `devflow::setup_of`, `devflow::set_of`, the asks — so the screen holds no
//! rule of its own and the file stays the source of truth.

use std::path::{Path, PathBuf};

use devpit_core::{preference, Store};
use devpit_devflow::config::{Config, StageGate, Step};
use devpit_devflow::detect::{self, Detected};
use devpit_devflow::folder::{State, FILE};
use devpit_devflow::layers::Layered;
use devpit_devflow::preset::{self, Preset};
use devpit_devflow::{skills, Ask};
use devpit_rpc::{
    DevflowAsks, DevflowChange, DevflowCheck, DevflowCli, DevflowConfig, DevflowDetected,
    DevflowError, DevflowFileChange, DevflowFound, DevflowHint, DevflowPrefs, DevflowPreview,
    DevflowReviewer, DevflowScreen, DevflowSkill, DevflowSkillText, DevflowStage, DevflowState,
    DevflowTeam, ErrorCode, Project, RpcError,
};
use serde_json::{json, Map, Value};
use tauri::AppHandle;

use crate::devflow::{self, Offered};
use crate::devflow_asks::{self as asks, globally_on, made};

/// The most of a value or a set the window sends, checked before parsing.
const MOST_SENT: usize = 64 * 1024;

/// A project by its id, without asking git about its checkouts: nothing here
/// needs them.
pub(crate) fn project_here(store: &Store, project_id: &str) -> Result<Project, RpcError> {
    let (row, _) = crate::projects::locate(store, project_id)?;
    Ok(crate::projects::plain(store, row))
}

/// The person's own Devflow skills, under this devpit's home.
fn personal() -> PathBuf {
    Store::root()
        .map(|root| devpit_core::home::devflow_skills(&root))
        .unwrap_or_default()
}

/// What the screen shows for a file in `state`: the MCP's own state word.
pub(crate) fn state_of(state: &State, active: bool) -> DevflowState {
    match devflow::state_word(state, active) {
        "no file" => DevflowState::Missing,
        "invalid" => DevflowState::Invalid,
        "active" => DevflowState::Active,
        _ => DevflowState::Disabled,
    }
}

/// The screen for `project`, read now.
pub(crate) fn screen(app: Option<&AppHandle>, store: &Store, project: &Project) -> DevflowScreen {
    let root = Path::new(&project.root_path);
    let state = devflow::looked(app, store, project, root);
    let globally = globally_on(store);
    let active = matches!(
        devflow::offered(globally, &state, made(store, &project.id, Ask::Enable)),
        Offered::Active(_)
    );
    let (yaml, config, error) = match &state {
        State::Missing => (None, None, None),
        State::Invalid(text, invalid) => (
            text.clone(),
            None,
            Some(DevflowError {
                line: invalid.line.and_then(|line| u32::try_from(line).ok()),
                field: invalid.field.clone(),
                message: invalid.message.clone(),
            }),
        ),
        State::Valid(text, config) => {
            let layered = devflow::layered(root);
            let personal = personal();
            let places = SkillsIn {
                root,
                personal: &personal,
                layered: layered.as_ref(),
            };
            (
                Some(text.clone()),
                Some(view(config, Some(&places), &[])),
                None,
            )
        }
    };
    let detected = matches!(state, State::Missing).then(|| {
        let found = detect::detected(root, devflow::machine(store, &project.id));
        detected_view(&found)
    });
    DevflowScreen {
        project_id: project.id.clone(),
        state: state_of(&state, active),
        globally_on: globally,
        file: root.join(FILE).display().to_string(),
        yaml,
        config,
        error,
        waiting: asks::listed()
            .asks
            .into_iter()
            .filter(|one| one.project_id == project.id)
            .collect(),
        detected,
        preset: asks::preset(store).word().to_owned(),
    }
}

fn skill_view(name: &str, resolved: Result<skills::Skill, String>) -> DevflowSkill {
    match resolved {
        Ok(skill) => DevflowSkill {
            name: name.to_owned(),
            layer: skill.layer.word().to_owned(),
            file: skill.file.map(|file| file.display().to_string()),
            refused: None,
        },
        Err(why) => DevflowSkill {
            name: name.to_owned(),
            layer: String::new(),
            file: None,
            refused: Some(why),
        },
    }
}

/// Where the screen reads skills from: the project, the person's folder, and
/// the layers that apply.
pub(crate) struct SkillsIn<'a> {
    pub root: &'a Path,
    pub personal: &'a Path,
    pub layered: Option<&'a Layered>,
}

fn stage_skill(step: &Step, places: &SkillsIn) -> DevflowSkill {
    let teams = places
        .layered
        .map(|layered| layered.applied.as_slice())
        .unwrap_or_default();
    match step.builtin() {
        Some(stage) => {
            let name = format!("devflow-{stage}");
            skill_view(
                &name,
                skills::resolve_with(places.root, places.personal, teams, &name),
            )
        }
        None => {
            let installed = devflow::installed_folder();
            let found = skills::own(
                &step.uses,
                &skills::Places {
                    project: places.root,
                    personal: places.personal,
                    teams,
                    installed: installed.as_deref(),
                },
            );
            match found {
                Ok(found) => DevflowSkill {
                    name: step.uses.clone(),
                    layer: found.layer,
                    file: found.file.map(|file| file.display().to_string()),
                    refused: None,
                },
                Err(why) => DevflowSkill {
                    name: step.uses.clone(),
                    layer: String::new(),
                    file: None,
                    refused: Some(why),
                },
            }
        }
    }
}

/// `config` as the screen draws it. `skills_in` resolves each stage's skill;
/// `sources` says where a check's command was found.
pub(crate) fn view(
    config: &Config,
    skills_in: Option<&SkillsIn>,
    sources: &[(String, String)],
) -> DevflowConfig {
    let stages = &config.stages;
    let check = |one: &devpit_devflow::config::Check| DevflowCheck {
        name: one.name.clone(),
        run: one.run.clone(),
        report: one.report.word().to_owned(),
        evidence: one
            .evidence
            .iter()
            .map(|kept| kept.word().to_owned())
            .collect(),
        source: sources
            .iter()
            .find(|(command, _)| *command == one.run)
            .map(|(_, source)| source.clone()),
    };
    let staged = |step: &Step| {
        let builtin = step.builtin();
        let own = step.own.as_ref();
        DevflowStage {
            name: step.name().to_owned(),
            uses: step.uses.clone(),
            own: builtin.is_none(),
            enabled: config.is_on(step),
            approve: match builtin {
                Some("briefing") => Some(stages.briefing.approve.word().to_owned()),
                Some("plan") => Some(stages.plan.approve.word().to_owned()),
                Some(_) => None,
                None => own
                    .and_then(|own| own.approve)
                    .map(|approve| approve.word().to_owned()),
            },
            publish: (builtin == Some("plan"))
                .then_some(stages.plan.publish)
                .flatten()
                .map(|publish| publish.word().to_owned()),
            checks: if builtin == Some("execution") {
                stages.execution.checks.iter().map(check).collect()
            } else {
                Vec::new()
            },
            e2e_dir: (builtin == Some("execution")).then(|| stages.execution.e2e_dir.clone()),
            gate: match (builtin, own.and_then(|own| own.gate.as_ref())) {
                (Some("completion"), _) => Some(stages.completion.gate.clone()),
                (None, Some(StageGate::Rubric(rubric))) => Some(rubric.clone()),
                _ => None,
            },
            command: match own.and_then(|own| own.gate.as_ref()) {
                Some(StageGate::Command(command)) => Some(command.clone()),
                _ => None,
            },
            rubric: config.decisions.rubrics.get(step.name()).cloned(),
            skill: skills_in.map(|places| stage_skill(step, places)),
        }
    };
    let layered = skills_in.and_then(|places| places.layered);
    DevflowConfig {
        enabled: config.enabled,
        version: u32::try_from(config.version).unwrap_or_default(),
        teams: layered
            .map(|layered| {
                layered
                    .teams
                    .iter()
                    .map(|team| DevflowTeam {
                        name: team.name.clone(),
                        git: team.git.clone(),
                        wanted: team.wanted.clone(),
                        locked_ref: team.locked_ref.clone(),
                        commit: team.commit.clone(),
                        state: team.state.to_owned(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        locked: layered.map_or_else(|| config.locked.clone(), |layered| layered.locks.clone()),
        stages_from: layered.map_or("project".to_owned(), |layered| layered.stages_from.clone()),
        stages: config.flow.iter().map(staged).collect(),
        reviewers: config
            .review
            .reviewers
            .iter()
            .map(|one| DevflowReviewer {
                agent: one.agent.word().to_owned(),
                persona: one.persona.clone(),
                model: one.model.clone(),
            })
            .collect(),
        block_on: config
            .review
            .block_on
            .iter()
            .map(|one| one.word().to_owned())
            .collect(),
        consolidate: config.review.consolidate.word().to_owned(),
        decisions_use: config.decisions.use_,
        decisions_mode: config.decisions.mode.word().to_owned(),
    }
}
/// What detection found, with the recommended setup a Custom one starts from.
pub(crate) fn detected_view(found: &Detected) -> DevflowDetected {
    let kinds = [
        ("test", &found.test),
        ("e2e", &found.e2e),
        ("lint", &found.lint),
    ];
    let sources: Vec<(String, String)> = preset::checks(found)
        .into_iter()
        .filter_map(|one| Some((one.run, one.source?)))
        .collect();
    DevflowDetected {
        found: kinds
            .iter()
            .flat_map(|(kind, all)| {
                all.iter().map(|one| DevflowFound {
                    kind: (*kind).to_owned(),
                    command: one.command.clone(),
                    source: one.source.clone(),
                })
            })
            .collect(),
        e2e_tools: found
            .e2e_tools
            .iter()
            .map(|tool| format!("{} ({})", tool.name, tool.source))
            .collect(),
        clis: found
            .clis
            .iter()
            .map(|cli| DevflowCli {
                name: cli.name.word().to_owned(),
                installed: cli.installed,
                signed_in: cli.signed_in,
            })
            .collect(),
        decisions_key: found.decisions_key,
        glossary: found.glossary.clone(),
        adrs: found.adrs.clone(),
        open: detect::open(found),
        hints: hints_said(&found.hints),
        recommended: view(
            &preset::starting(Preset::Recommended, found),
            None,
            &sources,
        ),
    }
}

/// How the repository already works, one kind each.
fn hints_said(hints: &devpit_devflow::detect::Hints) -> Vec<DevflowHint> {
    [
        ("Guides", &hints.guides),
        ("Pull request templates", &hints.pr_templates),
        ("Issue templates", &hints.issue_templates),
        ("Release branches", &hints.branches),
        ("Ticket keys", &hints.ticket_keys),
        ("Skills", &hints.skills),
    ]
    .into_iter()
    .filter(|(_, items)| !items.is_empty())
    .map(|(label, items)| DevflowHint {
        label: label.to_owned(),
        items: items.clone(),
    })
    .collect()
}

/// A JSON text the window sent, read only below the ceiling.
fn sent(text: &str, what: &str) -> Result<Value, RpcError> {
    if text.len() > MOST_SENT {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!("{what} is larger than {} KB", MOST_SENT / 1024),
        ));
    }
    serde_json::from_str(text)
        .map_err(|err| RpcError::new(ErrorCode::Invalid, format!("{what} is not JSON: {err}")))
}

fn setup_params(preset: &str, set: &str, apply: bool) -> Result<Value, RpcError> {
    let set = match sent(set, "the setup")? {
        Value::Object(set) => set,
        Value::Null => Map::new(),
        _ => {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "the setup is an object of dotted path to value",
            ))
        }
    };
    Ok(json!({ "preset": preset, "set": set, "apply": apply }))
}

fn refused(why: String) -> RpcError {
    RpcError::new(ErrorCode::Invalid, why)
}

/// `devflow_setup` as the MCP answers it, for the screen.
pub(crate) fn setup_now(
    app: Option<&AppHandle>,
    store: &Store,
    project: &Project,
    params: &Value,
) -> Result<Value, RpcError> {
    let root = Path::new(&project.root_path);
    let state = devflow::looked(app, store, project, root);
    devflow::setup_of(app, store, project, root, &state, params).map_err(refused)
}

/// `devflow_set` with `apply`, as the MCP answers it, for the screen.
pub(crate) fn edit_now(
    app: Option<&AppHandle>,
    store: &Store,
    project: &Project,
    path: &str,
    value: Value,
) -> Result<Value, RpcError> {
    let root = Path::new(&project.root_path);
    let state = devflow::looked(app, store, project, root);
    let params = json!({ "path": path, "value": value, "apply": true });
    let answer = devflow::set_of(app, store, project, root, &state, &params).map_err(refused)?;
    // Held back for the click: the screen offers that click instead.
    if answer["applied"] != true
        && answer["asks"]
            .as_array()
            .is_some_and(|asks| !asks.is_empty())
    {
        return Err(refused(
            answer["note"]
                .as_str()
                .unwrap_or_default()
                .trim()
                .to_owned(),
        ));
    }
    Ok(answer)
}

/// `devflow.screen` — a project's Devflow, read now.
#[tauri::command]
#[specta::specta]
pub async fn devflow_screen(app: AppHandle, project_id: String) -> Result<DevflowScreen, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let project = project_here(&store, &project_id)?;
        Ok(screen(Some(&app), &store, &project))
    })
    .await
}

/// `devflow.preview` — what a setup would write; nothing is written.
#[tauri::command]
#[specta::specta]
pub async fn devflow_preview(
    app: AppHandle,
    project_id: String,
    preset: String,
    set: String,
) -> Result<DevflowPreview, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let project = project_here(&store, &project_id)?;
        let params = setup_params(&preset, &set, false)?;
        let answer = setup_now(Some(&app), &store, &project, &params)?;
        let files = answer["files"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .map(|one| DevflowFileChange {
                path: one["path"].as_str().unwrap_or_default().to_owned(),
                change: one["change"].as_str().unwrap_or_default().to_owned(),
            })
            .collect();
        Ok(DevflowPreview {
            yaml: answer["yaml"].as_str().unwrap_or_default().to_owned(),
            files,
        })
    })
    .await
}

/// `devflow.create` — the person's Create: writes `.devpit/devflow/` as
/// `devpit_devflow_setup` would, and commits nothing.
#[tauri::command]
#[specta::specta]
pub async fn devflow_create(
    app: AppHandle,
    project_id: String,
    preset: String,
    set: String,
) -> Result<DevflowScreen, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let project = project_here(&store, &project_id)?;
        setup_now(
            Some(&app),
            &store,
            &project,
            &setup_params(&preset, &set, true)?,
        )?;
        asks::rang(Some(&app), &project.id);
        Ok(screen(Some(&app), &store, &project))
    })
    .await
}

/// `devflow.edit` — one field of devflow.yaml, as the person changed it.
#[tauri::command]
#[specta::specta]
pub async fn devflow_edit(
    app: AppHandle,
    project_id: String,
    path: String,
    value: String,
) -> Result<DevflowScreen, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let project = project_here(&store, &project_id)?;
        edit_now(
            Some(&app),
            &store,
            &project,
            &path,
            sent(&value, "the value")?,
        )?;
        Ok(screen(Some(&app), &store, &project))
    })
    .await
}

/// `devflow.ask` — the person asked to switch Devflow on, or to enforce:
/// the same confirmation an agent's request raises, so the click is one.
#[tauri::command]
#[specta::specta]
pub async fn devflow_ask(
    app: AppHandle,
    project_id: String,
    change: DevflowChange,
) -> Result<DevflowAsks, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let project = project_here(&store, &project_id)?;
        let ask = match change {
            DevflowChange::Enable => Ask::Enable,
            DevflowChange::Enforce => Ask::Enforce,
            // Asked about where they come from: a stage's gate, an epic.
            DevflowChange::Command | DevflowChange::Autopilot => {
                return Err(RpcError::new(
                    ErrorCode::Invalid,
                    "only switching Devflow on or enforcing is asked from here",
                ))
            }
        };
        asks::raise(Some(&app), &project.id, &project.name, ask, false);
        Ok(asks::listed())
    })
    .await
}

/// `devflow.skill_text` — a stage's skill, from the layer that has it.
#[tauri::command]
#[specta::specta]
pub async fn devflow_skill_text(
    project_id: String,
    name: String,
) -> Result<DevflowSkillText, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let project = project_here(&store, &project_id)?;
        let skill =
            skills::resolve(Path::new(&project.root_path), &personal(), &name).map_err(refused)?;
        Ok(DevflowSkillText {
            text: skill.text.clone(),
            skill: skill_view(&name, Ok(skill)),
        })
    })
    .await
}

fn prefs_of(store: &Store) -> DevflowPrefs {
    DevflowPrefs {
        enabled: globally_on(store),
        preset: asks::preset(store).word().to_owned(),
    }
}

/// `devflow.prefs` — Devflow's switch for every project, and where a new
/// setup starts.
#[tauri::command]
#[specta::specta]
pub async fn devflow_prefs() -> Result<DevflowPrefs, RpcError> {
    crate::off_main::blocking(|| Ok(prefs_of(&crate::projects::store()?))).await
}

/// `devflow.prefs_set` — either of them, the other left alone. Switching
/// Devflow tells every MCP server that what it offers changed.
#[tauri::command]
#[specta::specta]
pub async fn devflow_prefs_set(
    app: AppHandle,
    enabled: Option<bool>,
    preset: Option<String>,
) -> Result<DevflowPrefs, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        if let Some(word) = preset {
            let chosen = Preset::of(&word).ok_or_else(|| {
                RpcError::new(ErrorCode::Invalid, "a setup starts recommended or custom")
            })?;
            store.set_preference(preference::DEVFLOW_PRESET, chosen.word())?;
        }
        if let Some(on) = enabled {
            if on != globally_on(&store) {
                store.set_preference_flag(preference::DEVFLOW, on)?;
                asks::rang(Some(&app), "");
            }
        }
        Ok(prefs_of(&store))
    })
    .await
}

#[cfg(test)]
#[path = "devflow_screen_tests.rs"]
mod tests;
