//! One headless turn: run it, read the stream, report what it cost.
//!
//! This is the cheap half of the product. It takes no terminal, it declares a
//! spending cap before it starts, and the last line of its output says what it
//! actually spent — which is how a card stops saying "the agent is doing
//! something" and starts saying a number.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::{headless_argv, AgentError, PROGRAM};

/// What one turn produced.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    /// The final text, which is JSON when a schema was asked for.
    pub result: String,
    pub cost_usd: f64,
    pub duration_ms: i64,
    pub turns: i64,
    /// True when the CLI itself reported failure, budget included.
    pub is_error: bool,
    /// Why it ended: the CLI's own word, kept rather than mapped so a new one
    /// arrives intact instead of being flattened into "failed".
    pub stop_reason: Option<String>,
}

/// How a turn is configured.
pub struct Turn<'a> {
    pub prompt: &'a str,
    pub cwd: &'a Path,
    /// The `--agents` argument, when the step names an agent.
    pub agents: Option<&'a str>,
    /// A JSON Schema the answer has to satisfy.
    pub schema: Option<&'a str>,
    pub budget_usd: Option<f64>,
    pub model: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum Line {
    #[serde(rename = "result")]
    Result {
        #[serde(default)]
        result: String,
        #[serde(default)]
        is_error: bool,
        #[serde(default)]
        total_cost_usd: f64,
        #[serde(default)]
        duration_ms: i64,
        #[serde(default)]
        num_turns: i64,
        #[serde(default)]
        stop_reason: Option<String>,
    },
    #[serde(other)]
    Other,
}

/// Runs the turn and reports the outcome.
///
/// `on_partial` is called with each assistant fragment as it arrives, so a
/// card can show work in progress rather than a spinner. A turn that produces
/// no `result` line is a failure with the stderr attached: it means the CLI
/// died rather than answered.
pub fn run_turn(turn: &Turn<'_>, mut on_partial: impl FnMut(&str)) -> Result<Outcome, AgentError> {
    let argv = headless_argv(turn.agents, turn.schema, turn.budget_usd, turn.model);
    let mut child = Command::new(PROGRAM)
        .args(&argv[1..])
        .current_dir(turn.cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| AgentError::NotInstalled)?;

    {
        let mut stdin = child.stdin.take().ok_or(AgentError::NotInstalled)?;
        let message = serde_json::json!({
            "type": "user",
            "message": { "role": "user", "content": [{ "type": "text", "text": turn.prompt }] }
        });
        writeln!(stdin, "{message}").map_err(|err| AgentError::Unreadable(err.to_string()))?;
    }

    let stdout = child.stdout.take().ok_or(AgentError::NotInstalled)?;
    let mut outcome = None;
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        match serde_json::from_str::<Line>(&line) {
            Ok(Line::Result {
                result,
                is_error,
                total_cost_usd,
                duration_ms,
                num_turns,
                stop_reason,
            }) => {
                outcome = Some(Outcome {
                    result,
                    cost_usd: total_cost_usd,
                    duration_ms,
                    turns: num_turns,
                    is_error,
                    stop_reason,
                });
            }
            Ok(Line::Other) => on_partial(&line),
            // A line we cannot read is not a reason to abandon the run: the
            // one that matters is `result`, and it comes last.
            Err(_) => {}
        }
    }

    let status = child
        .wait()
        .map_err(|err| AgentError::Unreadable(err.to_string()))?;

    outcome.ok_or_else(|| AgentError::Failed {
        command: "headless turn".to_owned(),
        stderr: format!("the run ended with {status} before reporting a result"),
    })
}

/// Whether the answer satisfies the schema the step declared.
///
/// Deliberately shallow: required keys and their types, not a full JSON Schema
/// engine. A step's schema is written next to the step, and the failure that
/// matters is "the agent answered prose where the card expected fields".
pub fn validates(answer: &str, schema: &str) -> Result<(), String> {
    let answer: serde_json::Value =
        serde_json::from_str(answer).map_err(|_| "the answer is not JSON".to_owned())?;
    let schema: serde_json::Value =
        serde_json::from_str(schema).map_err(|_| "the schema is not JSON".to_owned())?;

    let Some(required) = schema.get("required").and_then(|r| r.as_array()) else {
        return Ok(());
    };
    let properties = schema.get("properties");

    for key in required.iter().filter_map(|k| k.as_str()) {
        let Some(value) = answer.get(key) else {
            return Err(format!("the answer has no `{key}`"));
        };
        let expected = properties
            .and_then(|p| p.get(key))
            .and_then(|p| p.get("type"))
            .and_then(|t| t.as_str());
        let matches = match expected {
            Some("string") => value.is_string(),
            Some("number") => value.is_number(),
            Some("boolean") => value.is_boolean(),
            Some("array") => value.is_array(),
            Some("object") => value.is_object(),
            _ => true,
        };
        if !matches {
            return Err(format!(
                "`{key}` should be {} and is {value}",
                expected.unwrap_or("something else")
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCHEMA: &str = r#"{"type":"object",
        "properties":{"verdict":{"type":"string"},"findings":{"type":"array"}},
        "required":["verdict","findings"]}"#;

    #[test]
    fn an_answer_with_the_declared_fields_passes() {
        assert_eq!(
            validates(r#"{"verdict":"approved","findings":[]}"#, SCHEMA),
            Ok(())
        );
    }

    /// The failure this exists to catch: prose where the card expected fields.
    #[test]
    fn prose_where_json_was_asked_for_is_named_as_such() {
        let error = validates("Looks good to me!", SCHEMA).expect_err("should refuse");
        assert!(error.contains("not JSON"), "{error}");
    }

    #[test]
    fn a_missing_required_field_is_named() {
        let error = validates(r#"{"verdict":"approved"}"#, SCHEMA).expect_err("should refuse");
        assert!(error.contains("findings"), "{error}");
    }

    #[test]
    fn a_field_of_the_wrong_type_is_named() {
        let error =
            validates(r#"{"verdict":"ok","findings":"none"}"#, SCHEMA).expect_err("should refuse");
        assert!(
            error.contains("findings") && error.contains("array"),
            "{error}"
        );
    }

    /// A step without a schema is a step that wanted prose.
    #[test]
    fn a_schema_without_required_fields_accepts_anything() {
        assert_eq!(
            validates(r#"{"anything":1}"#, r#"{"type":"object"}"#),
            Ok(())
        );
    }

    /// The stream is what the product reads, so the recorded shape is the
    /// test: `result` carries the cost, and it is the last line.
    #[test]
    fn the_result_line_carries_what_a_card_shows() {
        let line = r#"{"type":"result","subtype":"success","is_error":false,
            "result":"PONG","total_cost_usd":0.101315,"duration_ms":2303,"num_turns":1,
            "stop_reason":"end_turn"}"#;
        match serde_json::from_str::<Line>(line).expect("decode") {
            Line::Result {
                result,
                total_cost_usd,
                duration_ms,
                num_turns,
                is_error,
                stop_reason,
            } => {
                assert_eq!(result, "PONG");
                assert!((total_cost_usd - 0.101315).abs() < 1e-9);
                assert_eq!(duration_ms, 2303);
                assert_eq!(num_turns, 1);
                assert!(!is_error);
                assert_eq!(stop_reason.as_deref(), Some("end_turn"));
            }
            Line::Other => panic!("the result line decoded as something else"),
        }
    }

    /// Everything that is not the result is a fragment to show, not an error.
    #[test]
    fn the_other_lines_are_fragments_not_failures() {
        for line in [
            r#"{"type":"system","subtype":"init","session_id":"x"}"#,
            r#"{"type":"assistant","message":{"content":[]}}"#,
            r#"{"type":"rate_limit_event"}"#,
        ] {
            assert!(matches!(
                serde_json::from_str::<Line>(line),
                Ok(Line::Other)
            ));
        }
    }

    /// The whole path, against the installed CLI.
    ///
    /// Opt-in rather than skip-if-missing, and the difference matters: this
    /// one spends money. A test that quietly bills someone on every `make
    /// test` is a test that gets deleted. Run it with
    /// `QUOCKPIT_LIVE_TURN=1 cargo test -p quockpit-agentcli`.
    #[test]
    fn a_real_turn_answers_the_schema_and_reports_its_cost() {
        if std::env::var_os("QUOCKPIT_LIVE_TURN").is_none() {
            eprintln!("skipped: set QUOCKPIT_LIVE_TURN=1 to spend money on this one");
            return;
        }

        let mut fragments = 0;
        let outcome = run_turn(
            &Turn {
                prompt: "Reply with verdict \"approved\" and an empty findings array.",
                cwd: Path::new("/tmp"),
                agents: None,
                schema: Some(SCHEMA),
                budget_usd: Some(0.50),
                model: Some("claude-haiku-4-5-20251001"),
            },
            |_| fragments += 1,
        )
        .expect("the turn ran");

        assert!(!outcome.is_error, "{outcome:?}");
        assert_eq!(validates(&outcome.result, SCHEMA), Ok(()));
        assert!(outcome.cost_usd > 0.0, "a turn that cost nothing");
        assert!(fragments > 0, "nothing streamed while it worked");
    }
}
