//! The stream shape, tested beside the code that reads it.

use super::*;
use crate::schema::validates;

const SCHEMA: &str = r#"{"type":"object",
        "properties":{"verdict":{"type":"string"},"findings":{"type":"array"}},
        "required":["verdict","findings"]}"#;

/// The whole path, against the installed CLI.
///
/// Opt-in rather than skip-if-missing, and the difference matters: this
/// one spends money. A test that quietly bills someone on every `make
/// test` is a test that gets deleted. Run it with
/// `DEVPIT_LIVE_TURN=1 cargo test -p devpit-agentcli`.
#[test]
fn a_real_turn_answers_the_schema_and_reports_its_cost() {
    if std::env::var_os("DEVPIT_LIVE_TURN").is_none() {
        eprintln!("skipped: set DEVPIT_LIVE_TURN=1 to spend money on this one");
        return;
    }

    let mut fragments = 0;
    let outcome = run_turn(
        &Turn {
            env: &[],
            runner: None,
            prompt: "Reply with verdict \"approved\" and an empty findings array.",
            cwd: Path::new("/tmp"),
            agents: None,
            schema: Some(SCHEMA),
            budget_usd: Some(0.50),
            model: Some("claude-haiku-4-5-20251001"),
            settings: None,
            session_id: None,
        },
        |_| fragments += 1,
    )
    .expect("the turn ran");

    assert!(!outcome.is_error, "{outcome:?}");
    assert_eq!(validates(&outcome.result, SCHEMA), Ok(()));
    assert!(outcome.cost_usd > 0.0, "a turn that cost nothing");
    assert!(fragments > 0, "nothing streamed while it worked");
}
