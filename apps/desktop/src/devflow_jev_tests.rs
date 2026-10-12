use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex as Shared};

use devpit_core::store::decisions::DecisionWrite;
use devpit_devflow::config::Mode;

use super::*;

/// A provider on loopback answering every request, each question by
/// `answer(id)`, and keeping what it was asked.
fn provider(answer: fn(&str) -> Value) -> (String, Arc<Shared<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let base = format!(
        "http://127.0.0.1:{}/api/v1",
        listener.local_addr().expect("addr").port()
    );
    let heard = Arc::new(Shared::new(Vec::new()));
    let kept = Arc::clone(&heard);
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().expect("clone"));
            let mut length = 0;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        length = value.trim().parse().unwrap_or(0);
                    }
                }
                if line == "\r\n" {
                    break;
                }
            }
            let mut body = vec![0; length];
            let _ = reader.read_exact(&mut body);
            let asked: Value = serde_json::from_slice(&body).unwrap_or_default();
            let answers: serde_json::Map<String, Value> = asked["questions"]
                .as_object()
                .map(|questions| {
                    questions
                        .keys()
                        .map(|id| (id.clone(), answer(id)))
                        .collect()
                })
                .unwrap_or_default();
            if let Ok(mut all) = kept.lock() {
                all.push(asked);
            }
            let reply = json!({ "answers": answers, "usage": { "cost": 0.0001 } }).to_string();
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{reply}",
                reply.len()
            );
        }
    });
    (base, heard)
}

fn noul(p: f64) -> Value {
    json!({ "type": "noul", "noul": p })
}

/// Sure of everything, except that a plan's criteria can be tested.
fn untestable_plan(id: &str) -> Value {
    match id {
        "testable" => noul(0.2),
        "duplicate" => noul(0.05),
        "severity" => {
            json!({ "type": "choice", "choice": "low", "probabilities": { "low": 0.9, "high": 0.1 } })
        }
        _ => noul(0.95),
    }
}

fn conf(base: &str, key: Option<&str>) -> Conf {
    Conf {
        base: base.to_owned(),
        model: "typesafe/jev-1.13".to_owned(),
        cap_usd: 1.0,
        key: key.map(str::to_owned),
        ..Conf::default()
    }
}

fn log_store(dir: &Path) -> Store {
    Store::open(&dir.join("decisions.db")).expect("a store")
}

/// Each of the four rubrics asked of a provider on this machine: every
/// request asks for no retention and no training, carries the rubric's own
/// questions — done once per criterion — and never a diff; each answer is
/// judged by its thresholds and logged.
#[test]
fn the_four_rubrics_are_asked_with_no_retention_and_judged_by_their_thresholds() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (base, heard) = provider(untestable_plan);
    let criteria = [
        "opens the first link".to_owned(),
        "prints opened".to_owned(),
    ];
    let jev = Jev::with(
        log_store(dir.path()),
        conf(&base, Some("key")),
        "prj_jev",
        Some("card_1"),
        dir.path(),
        Mode::Shadow,
        &criteria,
    );
    assert!(jev.answers());
    let finding = json!({
        "file": "app.py", "line": 2, "severity": "high", "axis": "security", "title": "Shell injection",
        "description": "d", "evidence": "e", "poc": "p", "agent": "claude", "persona": "devflow-security",
    });
    let brief = devpit_devflow::brief::Brief {
        goal: Some("open links".into()),
        success: Some("tasks open 1 opens it".into()),
        out_of_scope: Some(vec!["previews".into()]),
        ..Default::default()
    };
    let slices: Vec<devpit_devflow::plan::Slice> = serde_json::from_value(json!([
        { "key": "s1", "title": "Open a link", "criteria": ["opens it"] }
    ]))
    .expect("a plan");
    let asked = [
        (
            jev::FINDING,
            jev::finding_state(&finding, &[], &[(1, 3)], &criteria),
        ),
        (
            jev::BRIEF,
            jev::brief_state("[devflow] Brief: Links", &brief),
        ),
        (jev::PLAN, jev::plan_state(&slices)),
        (jev::DONE, json!({ "criteria": criteria })),
    ];
    let outcomes: Vec<String> = asked
        .iter()
        .map(|(rubric, state)| {
            let said = jev.judge(rubric, state);
            assert!(said.decision.is_some(), "{rubric} was logged");
            assert_eq!(said.mode, Mode::Shadow);
            said.outcome
        })
        .collect();
    assert_eq!(outcomes, ["pass", "pass", "fail", "pass"]);

    let heard = heard.lock().expect("heard").clone();
    assert_eq!(heard.len(), 4);
    for request in &heard {
        assert_eq!(request["provider"]["zdr"], true);
        assert_eq!(request["provider"]["data_collection"], "deny");
        assert!(!request["state"].to_string().contains("diff --git"));
    }
    assert!(heard[0]["questions"].get("inDiff").is_some());
    assert!(heard[1]["questions"].get("outOfScope").is_some());
    assert!(heard[2]["questions"].get("vertical").is_some());
    assert!(heard[3]["questions"].get("criterion2").is_some());
    let logged = jev
        .store
        .lock()
        .expect("store")
        .decisions_latest(10)
        .expect("log");
    let mut rubrics: Vec<String> = logged.iter().filter_map(|row| row.rubric.clone()).collect();
    rubrics.sort();
    assert_eq!(
        rubrics,
        [
            "devflow-brief",
            "devflow-done",
            "devflow-finding",
            "devflow-plan"
        ]
    );
    assert!(logged.iter().all(|row| row.mode == "shadow"));
}

/// The gate's own rubric moved with the others: a project's
/// `.devpit/devflow/rubrics/` overrides devpit's, and `.devpit/rubrics/` no
/// longer does for the flow's own names.
#[test]
fn a_projects_own_rubric_in_its_devflow_folder_overrides_devpits() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let ours = r#"{ "questions": { "short": { "type": "noul", "instructions": "Is it short?" } }, "thresholds": { "short": { "min": 0.5 } } }"#;
    std::fs::create_dir_all(root.join(".devpit/rubrics")).expect("dir");
    std::fs::write(root.join(".devpit/rubrics/devflow-done.json"), ours).expect("old place");
    let done = rubric(root, jev::DONE, &["a".to_owned()]).expect("done");
    assert!(
        done.questions.get("criterion1").is_some(),
        "the old place is not read"
    );

    std::fs::create_dir_all(root.join(jev::DIR)).expect("dir");
    for name in [jev::DONE, jev::BRIEF] {
        std::fs::write(root.join(jev::DIR).join(format!("{name}.json")), ours).expect("write");
        let read = rubric(root, name, &[]).expect("ours");
        assert_eq!(read.name, name);
        assert!(read.questions.get("short").is_some());
    }
    let plan = rubric(root, jev::PLAN, &[]).expect("devpit's");
    assert!(plan.questions.get("dependencies").is_some());
    assert!(rubric(root, "../escape", &[]).is_err());
}

fn project(dir: &Path) -> (Store, Project) {
    let store = crate::projects::store().expect("store");
    let id = store.add_project(dir, None).expect("project");
    store.ensure_board(&id).expect("board");
    let project = serde_json::from_value(json!({
        "id": id, "name": id, "rootPath": dir.display().to_string(), "group": null,
        "accent": "#000000", "worktrees": [], "unreadable": null, "orchestrator": null,
        "origin": null, "lastOpenedAt": null, "icon": null, "color": null,
    }))
    .expect("a project");
    (store, project)
}

fn epic_of(project: &Project) -> String {
    let said = crate::devflow_plan::respond(
        None,
        "devflow_brief",
        project,
        &json!({
            "title": "Links", "goal": "Open a task's link", "success": "tasks open 1 opens it",
            "out_of_scope": ["previews"]
        }),
    )
    .expect("a brief");
    said["cardId"].as_str().expect("an id").to_owned()
}

/// The same plan, the same answer: in shadow Jev's "fail" is recorded and
/// the cards are made; acting, it holds them, and says why.
#[test]
fn shadow_records_the_plans_fail_and_enforce_holds_its_cards() {
    let (base, _heard) = provider(untestable_plan);
    let mut config = Config::default();
    config.decisions.use_ = true;
    for mode in [Mode::Shadow, Mode::Enforce] {
        let dir = tempfile::tempdir().expect("tempdir");
        let (store, project) = project(dir.path());
        let epic = epic_of(&project);
        let jev = Jev::with(
            log_store(dir.path()),
            conf(&base, Some("key")),
            &project.id,
            Some(&epic),
            dir.path(),
            mode,
            &[],
        );
        let plan = |apply: bool| {
            crate::devflow_plan::respond_judged(
                None,
                "devflow_plan",
                &project,
                &json!({ "epic": epic, "apply": apply, "cards": [
                    { "key": "s1", "title": "Open a link", "criteria": ["opens it"] }
                ] }),
                Some((&config, &jev)),
            )
        };
        let preview = plan(false).expect("a preview");
        assert_eq!(preview["jev"]["brief"]["outcome"], "pass", "{preview}");
        let applied = plan(true);
        let cards = store.cards(&project.id).expect("cards").len();
        match mode {
            Mode::Shadow => {
                let said = applied.expect("made");
                assert_eq!(said["jev"]["plan"]["outcome"], "fail", "{said}");
                assert_eq!(cards, 2, "the epic and its card");
            }
            Mode::Enforce => {
                let why = applied.expect_err("held");
                assert!(
                    why.contains("testable") && why.contains("nothing was created"),
                    "{why}"
                );
                assert_eq!(cards, 1, "only the epic");
            }
        }
    }
}

/// No key: nothing is asked, nothing acts, and the plan is made as it
/// would be without Decisions.
#[test]
fn without_a_key_devflow_works_as_it_does_without_jev() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (store, project) = project(dir.path());
    let epic = epic_of(&project);
    let mut config = Config::default();
    config.decisions.use_ = true;
    let jev = Jev::with(
        log_store(dir.path()),
        conf("http://127.0.0.1:9", None),
        &project.id,
        Some(&epic),
        dir.path(),
        Mode::Enforce,
        &[],
    );
    assert!(
        !jev.answers(),
        "a review falls back to a model pass or none"
    );
    let said = jev.judge(jev::FINDING, &json!({}));
    assert_eq!(said.outcome, "skipped");
    assert!(!said.acts());
    let ask = |apply: bool| {
        crate::devflow_plan::respond_judged(
            None,
            "devflow_plan",
            &project,
            &json!({ "epic": epic, "apply": apply, "cards": [{ "key": "s1", "title": "Open a link" }] }),
            Some((&config, &jev)),
        )
        .expect("planned")
    };
    ask(false);
    let made = ask(true);
    assert_eq!(made["jev"]["plan"]["outcome"], "skipped", "{made}");
    assert_eq!(store.cards(&project.id).expect("cards").len(), 2);
    assert!(jev
        .store
        .lock()
        .expect("store")
        .decisions_latest(5)
        .expect("log")
        .is_empty());
}

fn decision(store: &Store, project: &str, card: &str, rubric: &str, outcome: &str) -> String {
    store
        .log_decision(
            &DecisionWrite {
                project_id: Some(project),
                card_id: Some(card),
                gate: rubric,
                mode: "shadow",
                rubric: Some(rubric),
                rubric_version: Some("1"),
                questions: "{}",
                answers: Some(r#"{"real":{"type":"noul","noul":0.9}}"#),
                thresholds: Some(r#"{"real":{"min":0.7}}"#),
                outcome,
                cost_usd: 0.0,
                latency_ms: 1,
                state_digest: "d:1",
            },
            crate::deciding::now(),
        )
        .expect("logged")
}

fn verdict_of(store: &Store, card: &str, rubric: &str, id: &str) -> Option<String> {
    store
        .decisions_of_card(card, rubric)
        .expect("read")
        .into_iter()
        .find(|row| row.id == id)
        .and_then(|row| row.person_verdict)
}

/// What the person does is the label: reopening a card Jev called done says
/// it was not; calling a finding a false positive says it was not real, and
/// taking that back says it was; finishing the card says yes to what stood.
#[test]
fn reopening_a_card_and_a_false_positive_are_labels_on_jevs_decisions() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().canonicalize().expect("root");
    let (store, project) = project(&root);
    let columns = store.columns(&project.id).expect("columns");
    let role = |name: &str| {
        columns
            .iter()
            .find(|column| {
                devpit_core::store::column_roles::role_of(column.role.as_deref(), &column.name)
                    == Some(name)
            })
            .map(|column| column.id.clone())
            .expect(name)
    };
    let (done, doing) = (role("done"), role("doing"));
    let card = store
        .create_card(&project.id, &done, "[devflow] Open links", "")
        .expect("card");
    let here = root.display().to_string();
    store
        .set_card_front(&card, Some(&here), Some("HEAD"))
        .expect("its checkout");

    let called_done = decision(&store, &project.id, &card, jev::DONE, "pass");
    moved(&store, &project.id, &card, Some(&done), &doing);
    assert_eq!(
        verdict_of(&store, &card, jev::DONE, &called_done).as_deref(),
        Some("fail")
    );

    let real = decision(&store, &project.id, &card, jev::FINDING, "pass");
    let other = decision(&store, &project.id, &card, jev::FINDING, "pass");
    let (relative, path) =
        devpit_devflow::runs::file_in(&root, &card, Some("20261011T143005Z"), "findings.json")
            .expect("a run");
    assert!(relative.ends_with("findings.json"));
    let finding = |title: &str, decision: &str| {
        json!({
            "file": "app.py", "line": 2, "severity": "low", "axis": "security", "title": title,
            "description": "d", "evidence": "e", "agent": "claude", "persona": "devflow-security",
            "jev": { "decision": decision, "mode": "shadow", "outcome": "pass" },
        })
    };
    std::fs::write(
        &path,
        json!({ "version": 1, "head": "h", "verdict": "approve",
            "findings": [finding("one", &real), finding("two", &other)] })
        .to_string(),
    )
    .expect("findings.json");
    let run = format!("devflow:{card}/20261011T143005Z");
    crate::devflow_review::mark(&store, &run, 0, true).expect("a false positive");
    assert_eq!(
        verdict_of(&store, &card, jev::FINDING, &real).as_deref(),
        Some("fail")
    );
    crate::devflow_review::mark(&store, &run, 0, false).expect("taken back");
    assert_eq!(
        verdict_of(&store, &card, jev::FINDING, &real).as_deref(),
        Some("pass")
    );

    moved(&store, &project.id, &card, Some(&doing), &done);
    assert_eq!(
        verdict_of(&store, &card, jev::FINDING, &other).as_deref(),
        Some("pass"),
        "a finding left standing when the card was finished"
    );
}

fn row(rubric: &str, real: f64, outcome: &str, verdict: &str) -> DecisionRow {
    DecisionRow {
        id: format!("dcn_{real}"),
        at: 0,
        project_id: Some("prj".into()),
        card_id: Some("card".into()),
        gate: rubric.into(),
        mode: "shadow".into(),
        rubric: Some(rubric.into()),
        questions: "{}".into(),
        answers: Some(
            json!({ "real": noul(real), "inDiff": noul(0.99), "severity": { "choice": "low", "probabilities": { "low": 0.1 } } })
                .to_string(),
        ),
        thresholds: Some(
            r#"{"real":{"min":0.7,"grey":0.4},"inDiff":{"min":0.6},"severity":{"min":0.5},"duplicate":{"max":0.3}}"#
                .into(),
        ),
        outcome: outcome.into(),
        cost_usd: 0.0,
        latency_ms: 0,
        state_digest: "d".into(),
        person_verdict: (!verdict.is_empty()).then(|| verdict.to_owned()),
    }
}

/// Synthetic decisions: `devflow-finding` passed three findings the person
/// then called false positives, all scored under 0.8. Its accuracy says so,
/// and a higher `min` is suggested; a choice's low confidence is not what
/// its passing turned on.
#[test]
fn accuracy_and_a_suggested_threshold_come_from_the_labelled_decisions() {
    let mut rows = vec![
        row(jev::FINDING, 0.72, "pass", "fail"),
        row(jev::FINDING, 0.75, "pass", "fail"),
        row(jev::FINDING, 0.78, "pass", "fail"),
        row(jev::FINDING, 0.2, "fail", "fail"),
        row(jev::FINDING, 0.5, "grey", "pass"),
        row(jev::FINDING, 0.6, "grey", ""),
    ];
    for real in [0.9, 0.92, 0.95, 0.97] {
        rows.push(row(jev::FINDING, real, "pass", "pass"));
    }
    rows.push(row(jev::DONE, 0.9, "pass", ""));
    let all = calibration(&rows);
    assert_eq!(all[0].rubric, jev::FINDING, "the flow's order");
    let finding = &all[0];
    assert_eq!(finding.decisions, 10);
    assert_eq!(finding.labelled, 9);
    assert_eq!(finding.grey, 1);
    assert_eq!(finding.agreed, 5);
    assert_eq!(finding.accuracy, Some(5.0 / 8.0));
    assert_eq!(
        finding.min,
        Some(0.5),
        "the lowest min of its questions now"
    );
    let suggested = finding.suggested_min.expect("suggested");
    assert!(suggested > 0.78 && suggested <= 0.9, "{suggested}");
    assert_eq!(all[1].rubric, jev::DONE);
    assert_eq!(all[1].labelled, 0);
    assert!(all[1].note.is_some());
    assert_eq!(
        score_of(
            &json!({ "real": noul(0.8), "inDiff": noul(0.6), "severity": { "choice": "low", "probabilities": { "low": 0.1 } } }),
            &json!({ "real": { "min": 0.7 }, "inDiff": { "min": 0.6 }, "severity": { "min": 0.5 } })
        ),
        Some(0.6)
    );
}
