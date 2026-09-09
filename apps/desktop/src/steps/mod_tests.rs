//! The verdict and naming helpers, tested beside them.

use super::*;

const REVIEW: &str = r#"{"prompt":"review it","verdictField":"verdict","sendsBackWhen":"revise"}"#;

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

/// The chain, end to end, with a real agent at the far side of it.
///
/// Every piece has a test of its own; this is the one that proves they add
/// up — a card in a store, a step configured against it, the agent named from
/// the catalogue on disk, and a run that comes back with an answer the schema
/// accepts and a cost above zero.
///
/// Opt-in behind `DEVPIT_LIVE_TURN`: it spends money.
#[test]
fn a_card_and_a_step_produce_a_real_answer() {
    if std::env::var_os("DEVPIT_LIVE_TURN").is_none() {
        eprintln!("skipped: set DEVPIT_LIVE_TURN=1 to spend money on this one");
        return;
    }

    let dir = tempfile::tempdir().expect("tempdir");
    let store = Store::open(&dir.path().join("state.db")).expect("open");
    let project = store.add_project(dir.path(), None).expect("project");
    store.ensure_board(&project).expect("board");
    let column = store.columns(&project).expect("columns")[1].id.clone();

    let card = store
        .create_card(
            &project,
            &column,
            "Add a retry to the upload",
            "It fails on a flaky network and loses the file.",
        )
        .expect("card");

    // The agent the step names has to be one that is actually on disk.
    let catalogue = devpit_agentcli::read_agents(&agents_dir().expect("agents dir"));
    let Some(agent) = catalogue.agents.first() else {
        eprintln!("skipped: no agents seeded on this machine");
        return;
    };

    // The schema travels as a string inside the config, which is why it is
    // built separately rather than nested: the step stores JSON, and one of
    // its fields is itself JSON.
    let schema = r#"{"type":"object",
        "properties":{"verdict":{"type":"string"},"findings":{"type":"array"}},
        "required":["verdict","findings"]}"#;
    let config = serde_json::json!({
        "agent": agent.name,
        "prompt": "Reply with a verdict of \"approved\" and an empty findings array.",
        "schema": schema,
        "budgetUsd": 0.5,
        "model": "claude-haiku-4-5-20251001"
    })
    .to_string();

    let step_id = store
        .create_step(&project, "agent", "review it", &config, false)
        .expect("step");
    store
        .set_column_step(&column, Some(&step_id))
        .expect("attach");

    let step = devpit_rpc::Step {
        id: step_id.clone(),
        kind: devpit_rpc::StepKind::Agent,
        name: "review it".to_owned(),
        config,
        irreversible: false,
    };

    let mut streamed = 0;
    let finished =
        agent::run(&store, &card, &step, |_| streamed += 1, |_| {}).expect("the step ran");

    assert!(finished.ok, "the step failed: {}", finished.output);
    assert!(
        finished.cost_usd > 0.0,
        "a turn that cost nothing: {}",
        finished.output
    );
    assert!(streamed > 0, "nothing reached the card while it worked");

    // And the run lands on the card, which is where a person reads it.
    let run = store.start_run(&card, &step_id).expect("start");
    store
        .finish_run(
            &run,
            "ok",
            Some(&finished.output),
            Some(finished.cost_usd),
            Some(finished.duration_ms),
            None,
        )
        .expect("finish");

    assert_eq!(store.runs(&card).expect("runs").len(), 1);
    assert!(store.card_cost(&card).expect("cost") > 0.0);
}
