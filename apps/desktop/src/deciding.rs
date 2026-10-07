//! Asking Decisions: Jev, TypeSafe AI's "System One" model, which writes no
//! text — given a state and typed questions it answers each with a calibrated
//! probability, a distribution or a score.
//!
//! Optional everywhere it is used. Without a key, with the project opted out,
//! over the day's cap or with the provider unreachable, the answer is
//! [`Decided::Skipped`] and the caller does what it did before Decisions.
//!
//! Synchronous, for the blocking pool and the agent API's own threads: it must
//! not be called from inside the async runtime.

use std::time::{Duration, Instant};

use devpit_core::store::decisions::DecisionWrite;
use devpit_core::Store;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::rubric::{judged, Rubric};

/// About 32k tokens of state. Past it the state is cut, and the answer says so.
pub(crate) const MOST_STATE: usize = 100_000;

/// A decision is a gate in someone's flow; a slow one is a skipped one.
const WAIT: Duration = Duration::from_secs(5);

/// The most of a reply read from the provider.
const MOST_REPLY: usize = 512 * 1024;

/// What a decision needs that the settings hold.
#[derive(Debug, Clone, Default)]
pub(crate) struct Conf {
    pub base: String,
    pub model: String,
    pub cap_usd: f64,
    /// None is Decisions off.
    pub key: Option<String>,
    pub opted_out: Vec<String>,
    /// Gate to mode; a gate not named is `shadow`.
    pub modes: Value,
    /// Values taken out of the state before it leaves the machine.
    pub secrets: Vec<String>,
}

/// One question to ask.
pub(crate) struct Asking<'a> {
    pub project_id: Option<&'a str>,
    pub card_id: Option<&'a str>,
    pub gate: &'a str,
    pub rubric: Option<&'a Rubric>,
    pub state: &'a Value,
    pub questions: &'a Value,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Decided {
    Answered {
        answers: Value,
        cost_usd: f64,
        latency_ms: u64,
        truncated: bool,
    },
    Skipped(Skip),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Skip {
    /// No key: Decisions was never set up.
    Off,
    OptedOut,
    Budget,
    Failed(String),
}

impl Skip {
    pub(crate) fn reason(&self) -> String {
        match self {
            Self::Off => "Decisions is off — set it up in Settings → Decisions".to_owned(),
            Self::OptedOut => {
                "this project's state is not sent to Decisions — see Settings → Decisions"
                    .to_owned()
            }
            Self::Budget => {
                "today's Decisions budget is spent — raise it in Settings → Decisions".to_owned()
            }
            Self::Failed(why) => why.clone(),
        }
    }
}

/// What a gate decided: the worst verdict of its rubric, or `skipped`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GateResult {
    pub outcome: &'static str,
    pub answers: Value,
    pub verdicts: Value,
    pub mode: String,
    pub cost_usd: f64,
    pub latency_ms: u64,
    pub truncated: bool,
    pub skipped: Option<Skip>,
}

/// Asks with the settings on disk. See [`decide_in`].
pub(crate) fn decide(
    project_id: Option<&str>,
    gate: &str,
    state: Value,
    questions: Value,
) -> Decided {
    let asking = Asking {
        project_id,
        card_id: None,
        gate,
        rubric: None,
        state: &state,
        questions: &questions,
    };
    match ready() {
        Ok((store, conf)) => decide_in(&store, &conf, &asking, now()),
        Err(why) => Decided::Skipped(Skip::Failed(why)),
    }
}

/// A rubric's questions asked about `state`, and their answers judged by its
/// thresholds.
pub(crate) fn gate(
    project_id: Option<&str>,
    card_id: Option<&str>,
    name: &str,
    rubric: &Rubric,
    state: Value,
) -> GateResult {
    let asking = Asking {
        project_id,
        card_id,
        gate: name,
        rubric: Some(rubric),
        state: &state,
        questions: &rubric.questions,
    };
    match ready() {
        Ok((store, conf)) => gated(
            decide_in(&store, &conf, &asking, now()),
            rubric,
            mode_of(&conf.modes, name).to_owned(),
        ),
        Err(why) => gated(
            Decided::Skipped(Skip::Failed(why)),
            rubric,
            "shadow".to_owned(),
        ),
    }
}

fn ready() -> Result<(Store, Conf), String> {
    let store = Store::open_default().map_err(|err| err.to_string())?;
    let conf = crate::decisions::conf(&store)?;
    Ok((store, conf))
}

/// What a decision came to, as a gate reads it.
pub(crate) fn gated(decided: Decided, rubric: &Rubric, mode: String) -> GateResult {
    match decided {
        Decided::Answered {
            answers,
            cost_usd,
            latency_ms,
            truncated,
        } => {
            let (outcome, verdicts) = judged(rubric, &answers);
            GateResult {
                outcome: outcome.as_str(),
                answers: readable(&answers),
                verdicts,
                mode,
                cost_usd,
                latency_ms,
                truncated,
                skipped: None,
            }
        }
        Decided::Skipped(skip) => GateResult {
            outcome: "skipped",
            answers: Value::Null,
            verdicts: Value::Null,
            mode,
            cost_usd: 0.0,
            latency_ms: 0,
            truncated: false,
            skipped: Some(skip),
        },
    }
}

/// Asks once, without retrying, and logs what came of it.
///
/// Nothing is sent without a key, for a project opted out or past the day's
/// cap; those skips are not logged, because nothing was decided.
pub(crate) fn decide_in(store: &Store, conf: &Conf, asking: &Asking<'_>, now: i64) -> Decided {
    let Some(key) = conf.key.as_deref() else {
        return Decided::Skipped(Skip::Off);
    };
    if asking
        .project_id
        .is_some_and(|id| conf.opted_out.iter().any(|out| out == id))
    {
        return Decided::Skipped(Skip::OptedOut);
    }
    let spent = store
        .decision_spend_since(today(now))
        .map(|(spent, _)| spent)
        .unwrap_or(f64::INFINITY);
    if spent >= conf.cap_usd {
        return Decided::Skipped(Skip::Budget);
    }
    let (state, truncated) = prepared(asking.state, &conf.secrets);
    let questions = match wire_questions(asking.questions) {
        Ok(questions) => questions,
        Err(why) => return Decided::Skipped(Skip::Failed(why)),
    };
    let began = Instant::now();
    let called = call(&conf.base, &conf.model, key, &state, &questions);
    let latency_ms = began.elapsed().as_millis() as u64;
    let decided = match called {
        Ok((answers, cost_usd)) => Decided::Answered {
            answers,
            cost_usd,
            latency_ms,
            truncated,
        },
        Err(why) => Decided::Skipped(Skip::Failed(why)),
    };
    let (outcome, answers, cost_usd) = match &decided {
        Decided::Answered {
            answers, cost_usd, ..
        } => (
            asking
                .rubric
                .map_or("answered", |rubric| judged(rubric, answers).0.as_str()),
            Some(answers.to_string()),
            *cost_usd,
        ),
        Decided::Skipped(_) => ("skipped", None, 0.0),
    };
    let thresholds = asking
        .rubric
        .map(|rubric| json!(rubric.thresholds).to_string());
    let _ = store.log_decision(
        &DecisionWrite {
            project_id: asking.project_id,
            card_id: asking.card_id,
            gate: asking.gate,
            mode: mode_of(&conf.modes, asking.gate),
            rubric: asking.rubric.map(|rubric| rubric.name.as_str()),
            rubric_version: asking.rubric.and_then(|rubric| rubric.version.as_deref()),
            questions: &questions.to_string(),
            answers: answers.as_deref(),
            thresholds: thresholds.as_deref(),
            outcome,
            cost_usd,
            latency_ms: latency_ms as i64,
            state_digest: &digest(&state),
        },
        now,
    );
    decided
}

/// `enforce` only for a gate the settings name so; every other gate watches.
pub(crate) fn mode_of<'a>(modes: &'a Value, gate: &str) -> &'a str {
    match modes.get(gate).and_then(Value::as_str) {
        Some("enforce") => "enforce",
        _ => "shadow",
    }
}

/// The state as sent: text, with every secret devpit knows taken out, then cut
/// to [`MOST_STATE`] — in that order, so a cut cannot leave half a secret the
/// redaction no longer recognises.
pub(crate) fn prepared(state: &Value, secrets: &[String]) -> (String, bool) {
    let text = match state {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    };
    let clean = crate::kept_out::kept_out(&text, secrets);
    if clean.chars().count() <= MOST_STATE {
        return (clean, false);
    }
    (clean.chars().take(MOST_STATE).collect(), true)
}

/// The questions as the wire names them: assay's `boolean` is `noul` there.
pub(crate) fn wire_questions(questions: &Value) -> Result<Value, String> {
    let asked = questions
        .as_object()
        .filter(|asked| !asked.is_empty())
        .ok_or("questions are an object of question id to question")?;
    let mut wire = serde_json::Map::new();
    for (id, question) in asked {
        let mut question = question.clone();
        let kind = question.get("type").and_then(Value::as_str).unwrap_or("");
        let kind = match kind {
            "boolean" | "noul" => "noul",
            "choice" | "score" => kind,
            _ => return Err(format!("question `{id}` is noul, choice or score")),
        };
        question["type"] = json!(kind);
        wire.insert(id.clone(), question);
    }
    Ok(Value::Object(wire))
}

/// The answers with a noul's probability under a name an agent reads.
pub(crate) fn readable(answers: &Value) -> Value {
    let mut answers = answers.clone();
    if let Some(each) = answers.as_object_mut() {
        for answer in each.values_mut() {
            if let Some(p) = answer.get("noul").cloned() {
                answer["probability"] = p;
            }
        }
    }
    answers
}

/// A hash and a length: enough to tell two states apart, nothing to read.
pub(crate) fn digest(state: &str) -> String {
    let hash = Sha256::digest(state.as_bytes());
    let hex: String = hash.iter().take(8).map(|b| format!("{b:02x}")).collect();
    format!("{hex}:{}", state.chars().count())
}

/// One request to the provider's System One endpoint: the answers, and what
/// it cost.
pub(crate) fn call(
    base: &str,
    model: &str,
    key: &str,
    state: &str,
    questions: &Value,
) -> Result<(Value, f64), String> {
    // Every call: no retention and no training, whatever the account says.
    let body = json!({
        "model": model,
        "state": state,
        "questions": questions,
        "provider": { "zdr": true, "data_collection": "deny" },
    });
    let url = format!("{}/systemone", base.trim_end_matches('/'));
    let key = key.to_owned();
    tauri::async_runtime::block_on(async move {
        let client = reqwest::Client::builder()
            .timeout(WAIT)
            .user_agent(concat!("devpit/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|err| err.to_string())?;
        let mut reply = client
            .post(url)
            .bearer_auth(key)
            .json(&body)
            .send()
            .await
            .map_err(|err| format!("Decisions could not be reached: {err}"))?;
        let status = reply.status();
        let mut bytes = Vec::new();
        while let Some(chunk) = reply.chunk().await.map_err(|err| err.to_string())? {
            if bytes.len() + chunk.len() > MOST_REPLY {
                return Err("Decisions answered more than any answer needs".to_owned());
            }
            bytes.extend_from_slice(&chunk);
        }
        let read: Value = serde_json::from_slice(&bytes).unwrap_or_default();
        if !status.is_success() {
            let why = read["error"]["message"]
                .as_str()
                .unwrap_or("no reason given");
            return Err(format!("Decisions refused it ({status}): {why}"));
        }
        let answers = read
            .get("answers")
            .filter(|answers| answers.is_object())
            .cloned()
            .ok_or("Decisions sent no answers")?;
        let cost = read["usage"]["cost"].as_f64().unwrap_or(0.0);
        Ok((answers, cost))
    })
}

/// The start of `now`'s day, in UTC — the day the providers bill by.
pub(crate) fn today(now: i64) -> i64 {
    now - now.rem_euclid(86_400)
}

pub(crate) fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "deciding_tests.rs"]
mod tests;
