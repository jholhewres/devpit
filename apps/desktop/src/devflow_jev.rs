//! Jev in Devflow, on this machine: one of Devflow's rubrics asked of
//! Decisions for a project that uses it; the person's disagreements — a
//! card Jev called done and they reopened, a finding they called a false
//! positive — kept as labels on the decision log; and how each rubric agrees
//! with them, which Settings shows.
//!
//! The rubrics, their states and what an answer does are
//! `devpit_devflow::jev`'s; the thresholds a rubric would agree best at are
//! `calibrate`'s.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use devpit_core::store::decisions::DecisionRow;
use devpit_core::Store;
use devpit_devflow::calibrate::{self, Labelled};
use devpit_devflow::config::{Mode, GATE};
use devpit_devflow::jev::{self, Said};
use devpit_devflow::{Ask, Config};
use devpit_rpc::{DevflowCalibration, Project, RpcError, RubricAccuracy};
use serde_json::{json, Value};

use crate::deciding::{Asking, Conf};
use crate::rubric::Rubric;

/// `enforce` acts only once the person clicked for it.
pub(crate) fn mode_of(store: &Store, project_id: &str, config: &Config) -> Mode {
    if config.decisions.mode == Mode::Enforce
        && crate::devflow_asks::made(store, project_id, Ask::Enforce)
    {
        Mode::Enforce
    } else {
        Mode::Shadow
    }
}

/// One of Devflow's rubrics: the project's own in `.devpit/devflow/rubrics/`,
/// else devpit's — `devflow-done` asked once per criterion — else a rubric
/// of the project's `.devpit/rubrics/`, as `devpit_rubric_run` reads them.
pub(crate) fn rubric(root: &Path, name: &str, criteria: &[String]) -> Result<Rubric, String> {
    if let Some(text) = crate::agent_decide::rubric_text_in(root, jev::DIR, name)? {
        return crate::agent_decide::rubric_of(name, &text);
    }
    if name == GATE {
        return Ok(crate::devflow_gate::done_per_criterion(criteria));
    }
    match jev::BUILT_IN.iter().find(|(built, _)| *built == name) {
        Some((_, text)) => crate::agent_decide::rubric_of(name, text),
        None => crate::agent_decide::rubric_named(root, name),
    }
}

/// Jev, as one project's Devflow asks it.
pub(crate) struct Jev {
    store: Mutex<Store>,
    conf: Conf,
    project_id: String,
    card_id: Option<String>,
    root: PathBuf,
    mode: Mode,
    /// The card's criteria, which `devflow-done` asks one by one.
    criteria: Vec<String>,
}

impl Jev {
    /// Jev for this project's Devflow, wherever devflow.yaml says `use`:
    /// without a key, or opted out, each answer says it was skipped.
    pub(crate) fn of(
        project: &Project,
        config: &Config,
        root: &Path,
        card_id: Option<&str>,
        criteria: &[String],
    ) -> Option<Jev> {
        if !config.decisions.use_ {
            return None;
        }
        let store = crate::projects::store().ok()?;
        let conf = crate::decisions::conf(&store).ok()?;
        let mode = mode_of(&store, &project.id, config);
        Some(Jev::with(
            store,
            conf,
            &project.id,
            card_id,
            root,
            mode,
            criteria,
        ))
    }

    pub(crate) fn with(
        store: Store,
        conf: Conf,
        project_id: &str,
        card_id: Option<&str>,
        root: &Path,
        mode: Mode,
        criteria: &[String],
    ) -> Jev {
        Jev {
            store: Mutex::new(store),
            conf,
            project_id: project_id.to_owned(),
            card_id: card_id.map(str::to_owned),
            root: root.to_path_buf(),
            mode,
            criteria: criteria.to_vec(),
        }
    }

    /// Whether Decisions can answer at all: a key, the project not opted out.
    pub(crate) fn answers(&self) -> bool {
        self.conf.key.is_some() && !self.conf.opted_out.contains(&self.project_id)
    }

    /// The rubric `name` asked about `state`, logged under the mode it ran
    /// in, and judged by its thresholds.
    pub(crate) fn judge(&self, name: &str, state: &Value) -> Said {
        let skipped = |why: String| Said {
            rubric: name.to_owned(),
            decision: None,
            mode: self.mode,
            outcome: "skipped".to_owned(),
            verdicts: Value::Null,
            answers: Value::Null,
            skipped: Some(why),
        };
        let rubric = match rubric(&self.root, name, &self.criteria) {
            Ok(rubric) => rubric,
            Err(why) => return skipped(why),
        };
        let mut conf = self.conf.clone();
        // Logged under the flow's mode, not the Settings' gates.
        conf.modes = json!({ name: self.mode.word() });
        let Ok(store) = self.store.lock() else {
            return skipped("the decision log is busy".to_owned());
        };
        let asking = Asking {
            project_id: Some(&self.project_id),
            card_id: self.card_id.as_deref(),
            gate: name,
            rubric: Some(&rubric),
            state,
            questions: &rubric.questions,
        };
        let (decided, decision) =
            crate::deciding::decide_logged(&store, &conf, &asking, crate::deciding::now());
        let result = crate::deciding::gated(decided, &rubric, self.mode.word().to_owned());
        Said {
            rubric: name.to_owned(),
            decision,
            mode: self.mode,
            outcome: result.outcome.to_owned(),
            verdicts: result.verdicts,
            answers: result.answers,
            skipped: result.skipped.map(|skip| skip.reason()),
        }
    }
}

/// The rubric a project's completion is judged by.
fn completion_rubric(root: &Path) -> String {
    match crate::devflow::read(root) {
        devpit_devflow::State::Valid(_, config) => {
            jev::rubric_for(&config, "completion").unwrap_or_else(|| GATE.to_owned())
        }
        _ => GATE.to_owned(),
    }
}

/// Labels a decision with the person's verdict, saying what they did.
fn label(store: &Store, decision: &str, passing: bool, action: &str) {
    let _ = store.label_decision(decision, if passing { "pass" } else { "fail" }, action);
}

/// A card the person moved. Out of the done lane, its last word from the
/// completion rubric is a "no": they reopened what it judged. Into the done
/// lane, a word nobody labelled yet is a "yes", and so is each finding of the
/// card's last review still standing; one set aside, a "no".
pub(crate) fn moved(store: &Store, project_id: &str, card_id: &str, from: Option<&str>, to: &str) {
    let Ok(columns) = store.columns(project_id) else {
        return;
    };
    let done = |column: Option<&str>| {
        columns.iter().any(|one| {
            Some(one.id.as_str()) == column
                && devpit_core::store::column_roles::role_of(one.role.as_deref(), &one.name)
                    == Some("done")
        })
    };
    let (left_done, into_done) = (done(from), done(Some(to)));
    if left_done == into_done {
        return;
    }
    let Some(root) = checkout_of(store, card_id) else {
        return;
    };
    let last = store
        .decisions_of_card(card_id, &completion_rubric(&root))
        .ok()
        .and_then(|rows| rows.into_iter().next());
    if left_done {
        if let Some(last) = last {
            label(store, &last.id, false, "reopened");
        }
        return;
    }
    if let Some(last) = last.filter(|row| row.person_verdict.is_none()) {
        label(store, &last.id, true, "finished");
    }
    let Some((_, kept)) = crate::devflow_gate::newest(&root, card_id, "findings.json") else {
        return;
    };
    let unlabelled: Vec<String> = store
        .decisions_of_card(card_id, jev::FINDING)
        .unwrap_or_default()
        .into_iter()
        .filter(|row| row.person_verdict.is_none())
        .map(|row| row.id)
        .collect();
    for finding in kept["findings"].as_array().into_iter().flatten() {
        let Some(decision) = finding["jev"]["decision"].as_str() else {
            continue;
        };
        if unlabelled.iter().any(|id| id == decision) {
            let standing = devpit_devflow::findings::standing(finding);
            label(store, decision, standing, "finished");
        }
    }
}

/// The person's word on one finding Jev judged: a false positive is a "no"
/// to its being real; taking that back, or keeping one Jev set aside, a "yes".
pub(crate) fn finding_marked(store: &Store, finding: &Value, false_positive: bool) {
    let outcome = finding["jev"]["outcome"].as_str().unwrap_or_default();
    if !calibrate::decided(outcome) {
        return;
    }
    if let Some(decision) = finding["jev"]["decision"].as_str() {
        let action = if false_positive {
            "false_positive"
        } else {
            "kept"
        };
        label(store, decision, !false_positive, action);
    }
}

/// Where a card's work is: its own checkout, else its project's root.
pub(crate) fn checkout_of(store: &Store, card_id: &str) -> Option<PathBuf> {
    let card = store.card(card_id).ok()??;
    if let Some(tree) = card.worktree_path {
        return Some(PathBuf::from(tree));
    }
    let project = store.project_id_of_card(card_id).ok()??;
    Some(PathBuf::from(store.project(&project).ok()??.root_path))
}

/// What a decision's passing turned on: the lowest answer among its yes/no
/// questions held to a `min`.
pub(crate) fn score_of(answers: &Value, thresholds: &Value) -> Option<f64> {
    thresholds
        .as_object()?
        .iter()
        .filter(|(_, threshold)| threshold.get("min").is_some_and(Value::is_number))
        .filter_map(|(id, _)| {
            let answer = answers.get(id)?;
            // A choice's confidence is not how far it passed.
            answer
                .get("choice")
                .is_none()
                .then(|| crate::rubric::scalar(answer))
                .flatten()
        })
        .reduce(f64::min)
}

/// The lowest `min` among a decision's thresholds.
fn lowest_min(thresholds: &Value) -> Option<f64> {
    thresholds
        .as_object()?
        .values()
        .filter_map(|threshold| threshold.get("min")?.as_f64())
        .reduce(f64::min)
}

fn parsed(text: Option<&str>) -> Value {
    text.and_then(|text| serde_json::from_str(text).ok())
        .unwrap_or(Value::Null)
}

/// How each rubric a project was judged by agrees with the person: Devflow's
/// first, in the flow's order.
pub(crate) fn calibration(rows: &[DecisionRow]) -> Vec<RubricAccuracy> {
    let mut by_rubric: BTreeMap<&str, Vec<&DecisionRow>> = BTreeMap::new();
    for row in rows.iter().filter(|row| calibrate::decided(&row.outcome)) {
        if let Some(rubric) = row.rubric.as_deref() {
            by_rubric.entry(rubric).or_default().push(row);
        }
    }
    let order = |name: &str| {
        jev::STAGES
            .iter()
            .position(|(_, rubric)| *rubric == name)
            .unwrap_or(jev::STAGES.len())
    };
    let mut rubrics: Vec<(&str, Vec<&DecisionRow>)> = by_rubric.into_iter().collect();
    rubrics.sort_by_key(|(name, _)| order(name));
    rubrics
        .into_iter()
        .map(|(name, rows)| {
            let labelled: Vec<Labelled> = rows
                .iter()
                .filter_map(|row| {
                    Some(Labelled {
                        score: score_of(
                            &parsed(row.answers.as_deref()),
                            &parsed(row.thresholds.as_deref()),
                        ),
                        outcome: row.outcome.clone(),
                        truth: row.person_verdict.as_deref()? == "pass",
                    })
                })
                .collect();
            let calibrated = calibrate::calibrated(&labelled);
            let suggested = calibrated.suggested.as_ref();
            RubricAccuracy {
                rubric: name.to_owned(),
                decisions: rows.len() as u32,
                labelled: calibrated.labelled as u32,
                agreed: calibrated.agreed as u32,
                grey: calibrated.grey as u32,
                accuracy: calibrated.accuracy,
                min: rows
                    .first()
                    .and_then(|row| lowest_min(&parsed(row.thresholds.as_deref()))),
                suggested_min: suggested.map(|one| one.min),
                suggested_grey: suggested.and_then(|one| one.grey),
                suggested_accuracy: suggested.map(|one| one.accuracy),
                note: calibrated.note,
            }
        })
        .collect()
}

/// `devflow.calibration` — how each rubric agrees with the person in a
/// project, and the `min` and `grey` that would agree most.
#[tauri::command]
#[specta::specta]
pub async fn devflow_calibration(project_id: String) -> Result<DevflowCalibration, RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let rows = store.decisions_of_project(&project_id)?;
        Ok(DevflowCalibration {
            rubrics: calibration(&rows),
            project_id,
        })
    })
    .await
}

#[cfg(test)]
#[path = "devflow_jev_tests.rs"]
mod tests;
