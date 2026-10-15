use serde_json::{json, Value};

use super::*;

pub(crate) fn finding(file: &str, line: u64, severity: &str, title: &str) -> Value {
    let mut finding = json!({
        "file": file, "line": line, "severity": severity, "axis": "security",
        "title": title, "description": "why it matters", "evidence": "the quoted line",
        "agent": "claude", "persona": "devflow-security",
    });
    if matches!(severity, "critical" | "high") {
        finding["poc"] = json!("curl -d '<script>' /x");
    }
    finding
}

pub(crate) fn answer(agent: &str, persona: &str, findings: Vec<Value>) -> Value {
    let findings: Vec<Value> = findings
        .into_iter()
        .map(|mut finding| {
            finding["agent"] = json!(agent);
            finding["persona"] = json!(persona);
            finding
        })
        .collect();
    json!({
        "version": 1, "card": "card_1", "base": "b", "head": "h",
        "reviewers": [{ "agent": agent, "persona": persona }],
        "verdict": "approve",
        "findings": findings,
        "done_well": [format!("{agent} liked the tests")],
        "verification": { "tests": "ran them", "evidence": "the checks are green" },
    })
}

fn sorted(list: &Value) -> Vec<String> {
    let mut names: Vec<String> = match list {
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect(),
        Value::Object(fields) => fields.keys().cloned().collect(),
        _ => Vec::new(),
    };
    names.sort();
    names
}

/// The validator is written by hand: these are the places it must say what
/// the schema file says, read from the file itself.
#[test]
fn the_rules_here_are_the_schemas_own() {
    let schema: Value = serde_json::from_str(SCHEMA).expect("the schema is JSON");
    let finding = &schema["$defs"]["finding"];
    let mut required: Vec<String> = REQUIRED.iter().map(|one| (*one).to_owned()).collect();
    required.sort();
    assert_eq!(sorted(&finding["required"]), required);
    let mut fields: Vec<String> = FIELDS.iter().map(|one| (*one).to_owned()).collect();
    fields.sort();
    assert_eq!(sorted(&finding["properties"]), fields);
    let severities: Vec<&str> = Severity::ALL.iter().map(|one| one.word()).collect();
    assert_eq!(finding["properties"]["severity"]["enum"], json!(severities));
    assert_eq!(finding["properties"]["axis"]["enum"], json!(AXES));
    assert_eq!(
        sorted(&schema["required"]),
        [
            "done_well",
            "findings",
            "reviewers",
            "verdict",
            "verification",
            "version"
        ]
    );
}

#[test]
fn a_reviewers_answer_is_read_through_prose_and_a_fence() {
    let said = format!(
        "Here is the review:\n```json\n{}\n```\n",
        answer(
            "codex",
            "devflow-security",
            vec![finding("a.rs", 3, "low", "a name")]
        )
    );
    let read = read_answer(&said).expect("a good answer");
    assert_eq!(read["findings"][0]["file"], "a.rs");
}

/// Each answer breaks one rule of the schema, and each is refused saying so.
#[test]
fn output_that_breaks_the_schema_is_refused_with_what_breaks_it() {
    let good = answer(
        "claude",
        "devflow-security",
        vec![finding("a.rs", 3, "high", "xss")],
    );
    assert!(problems(&good).is_empty(), "{:?}", problems(&good));
    let broken = |change: &dyn Fn(&mut Value)| {
        let mut said = good.clone();
        change(&mut said);
        read_answer(&said.to_string()).expect_err("refused")
    };
    let no_poc = broken(&|said| {
        said["findings"][0]
            .as_object_mut()
            .expect("finding")
            .remove("poc");
    });
    assert!(no_poc.contains("needs a `poc`"), "{no_poc}");
    let severity = broken(&|said| said["findings"][0]["severity"] = json!("blocker"));
    assert!(severity.contains("severity"), "{severity}");
    let extra = broken(&|said| said["findings"][0]["confidence"] = json!(0.9));
    assert!(extra.contains("confidence"), "{extra}");
    let line = broken(&|said| said["findings"][0]["line"] = json!(0));
    assert!(line.contains("line"), "{line}");
    let well = broken(&|said| said["done_well"] = json!([]));
    assert!(well.contains("done_well"), "{well}");
    let axis = broken(&|said| said["findings"][0]["axis"] = json!("style"));
    assert!(axis.contains("axis"), "{axis}");
    assert!(read_answer("I found nothing worth saying.").is_err());
    assert!(read_answer("{ \"version\": 1, ").is_err());
}

#[test]
fn an_answer_is_signed_by_who_gave_it_and_carries_nothing_devpit_sets() {
    let mut said = answer(
        "claude",
        "devflow-security",
        vec![finding("a.rs", 3, "low", "x")],
    );
    said["findings"][0]["agent"] = json!("someone else");
    said["findings"][0]["false_positive"] = json!(true);
    let signed = signed(said, "codex", "devflow-security");
    assert_eq!(
        signed["reviewers"],
        json!([{ "agent": "codex", "persona": "devflow-security" }])
    );
    assert_eq!(signed["findings"][0]["agent"], "codex");
    assert!(signed["findings"][0].get("false_positive").is_none());
}

/// A headless reviewer that left out the names devpit fills was refused, and
/// its whole review was lost (B15).
#[test]
fn a_reviewer_devpit_ran_is_not_refused_for_the_names_devpit_fills() {
    let mut said = answer(
        "claude",
        "devflow-security",
        vec![finding("a.rs", 3, "medium", "fails open")],
    );
    for key in ["agent", "persona"] {
        said["findings"][0]
            .as_object_mut()
            .expect("finding")
            .remove(key);
    }
    said.as_object_mut().expect("answer").remove("reviewers");
    let read = read_signed(&said.to_string(), "claude", "devflow-security").expect("accepted");
    assert_eq!(read["findings"][0]["persona"], "devflow-security");
    said["findings"][0]["severity"] = json!("blocker");
    let refused = read_signed(&said.to_string(), "claude", "devflow-security")
        .expect_err("another break is still refused");
    assert!(refused.contains("severity"), "{refused}");
    assert!(read_signed("not json", "claude", "devflow-security").is_err());
}

fn all_of(answers: &[Value]) -> (Vec<Value>, Vec<Source>) {
    let all = answers
        .iter()
        .flat_map(|answer| answer["findings"].as_array().cloned().unwrap_or_default())
        .collect();
    let sources = answers
        .iter()
        .map(|answer| Source {
            agent: answer["reviewers"][0]["agent"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            persona: answer["reviewers"][0]["persona"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
        })
        .collect();
    (all, sources)
}

/// Three reviewers on one axis: the problem two of them found is kept once,
/// the worse of its two severities, saying 2 of 3; the one only the third
/// found says 1 of 3.
#[test]
fn the_same_problem_from_two_of_three_reviewers_is_kept_once_saying_so() {
    let persona = "devflow-security";
    let answers = [
        answer(
            "claude",
            persona,
            vec![finding("src/app.rs", 10, "high", "Shell injection in open")],
        ),
        answer(
            "codex",
            persona,
            vec![{
                let mut one = finding(
                    "src/app.rs",
                    9,
                    "critical",
                    "Shell injection when opening a link",
                );
                one["end_line"] = json!(12);
                one
            }],
        ),
        answer(
            "cursor",
            persona,
            vec![finding("src/store.rs", 40, "low", "Unused import")],
        ),
    ];
    let (all, sources) = all_of(&answers);
    let groups = grouped(&all, &|one, other| alike(&all[one], &all[other]));
    let merged = merged(&all, &groups, &sources);
    assert_eq!(merged.len(), 2, "{merged:#?}");
    assert_eq!(merged[0]["severity"], "critical");
    assert_eq!(merged[0]["agreement"], "2 of 3");
    assert_eq!(
        merged[0]["found_by"],
        json!([
            { "agent": "claude", "persona": persona },
            { "agent": "codex", "persona": persona }
        ])
    );
    assert_eq!(merged[1]["agreement"], "1 of 3");
    for finding in &merged {
        let mut wrapped = answer("x", persona, Vec::new());
        wrapped["findings"] = json!([finding]);
        assert!(problems(&wrapped).is_empty(), "{:?}", problems(&wrapped));
    }
}

#[test]
fn findings_are_one_only_in_the_same_file_and_axis_on_lines_that_meet() {
    let one = finding("a.rs", 10, "high", "Shell injection");
    let mut other_axis = one.clone();
    other_axis["axis"] = json!("spec");
    let mut far = one.clone();
    far["line"] = json!(30);
    let mut elsewhere = one.clone();
    elsewhere["file"] = json!("b.rs");
    for other in [other_axis, far, elsewhere] {
        assert!(!candidates(&one, &other), "{other}");
        assert!(!alike(&one, &other));
    }
    let worded_apart = finding("a.rs", 10, "high", "Missing test for rename");
    assert!(candidates(&one, &worded_apart));
    assert!(!alike(&one, &worded_apart), "the titles share nothing");
}

#[test]
fn jev_consolidates_only_where_decisions_answers_and_a_model_pass_needs_two_answers() {
    assert_eq!(consolidation(Consolidate::Jev, true, 1), Consolidate::Jev);
    assert_eq!(
        consolidation(Consolidate::Jev, false, 3),
        Consolidate::Agent
    );
    assert_eq!(
        consolidation(Consolidate::Jev, false, 1),
        Consolidate::Nothing
    );
    assert_eq!(
        consolidation(Consolidate::Agent, true, 2),
        Consolidate::Agent
    );
    assert_eq!(
        consolidation(Consolidate::Agent, true, 1),
        Consolidate::Nothing
    );
    assert_eq!(
        consolidation(Consolidate::Nothing, true, 3),
        Consolidate::Nothing
    );
}

#[test]
fn the_model_pass_answers_which_findings_are_one_and_nothing_out_of_range() {
    let same = agent_same("Sure: {\"same\": [[0, 2], [1, 3]]}", 4).expect("read");
    assert_eq!(same.len(), 2);
    assert!(same[0].contains(&2));
    assert_eq!(agent_same("{\"same\": [[0, 9]]}", 4), None);
    assert_eq!(agent_same("no idea", 4), None);
    let all = [
        finding("a.rs", 1, "low", "x"),
        finding("a.rs", 1, "low", "y"),
        finding("b.rs", 1, "low", "z"),
    ];
    let asked = agent_prompt(&all);
    assert!(asked.contains("0. [security] low a.rs:1-1"), "{asked}");
    assert!(
        !asked.contains("b.rs"),
        "a finding alone is not asked about"
    );
}

#[test]
fn a_critical_finding_blocks_until_the_person_calls_it_a_false_positive() {
    let block_on = [Severity::Critical, Severity::High];
    let mut list = vec![
        finding("a.rs", 1, "critical", "x"),
        finding("a.rs", 9, "low", "y"),
    ];
    assert_eq!(verdict(&list, &block_on), "request_changes");
    assert_eq!(blocking(&list, &block_on).len(), 1);
    list[0]["false_positive"] = json!(true);
    assert_eq!(verdict(&list, &block_on), "approve");
    assert_eq!(
        verdict(&[finding("a.rs", 1, "high", "x")], &[Severity::Critical]),
        "approve"
    );
}

#[test]
fn the_persons_false_positive_stays_on_the_finding_when_the_list_is_made_again() {
    let mut before = finding("a.rs", 4, "high", "x");
    before["false_positive"] = json!(true);
    let mut again = vec![
        finding("a.rs", 4, "high", "x"),
        finding("a.rs", 4, "high", "y"),
    ];
    carried(&mut again, &[before]);
    assert_eq!(again[0]["false_positive"], true);
    assert!(again[1].get("false_positive").is_none());
}

#[test]
fn the_headline_counts_what_still_stands() {
    let mut list = vec![
        finding("a.rs", 1, "critical", "x"),
        finding("a.rs", 9, "high", "y"),
    ];
    list[1]["false_positive"] = json!(true);
    let said = json!({ "verdict": "request_changes", "findings": list });
    assert_eq!(
        headline(&said),
        "[devflow] review: 1 findings (1 critical, 0 high) · verdict: request_changes"
    );
}

fn signed_one(finding: Value) -> Value {
    let said = answer("claude", "devflow-spec", vec![finding]);
    read_signed(&said.to_string(), "claude", "devflow-spec").expect("accepted")["findings"][0]
        .clone()
}

/// A missing test, a written rule or a style point cannot block a card on its
/// own: the ecopar review held a card for a test helper's panic.
#[test]
fn what_is_short_of_a_defect_is_held_to_its_ceiling_saying_why() {
    let mut missing = finding("src/pay.go", 40, "high", "no test for the mixed day");
    missing["kind"] = json!("missing_test");
    let read = signed_one(missing);
    assert_eq!(read["severity"], "medium");
    assert_eq!(read["capped"]["from"], "high");
    assert!(text(&read["capped"], "why").contains("missing test"));

    for file in [
        "pay/premiacao_test.go",
        "tests/PayTest.php",
        "web/src/pay.test.ts",
    ] {
        let read = signed_one(finding(file, 89, "high", "panics in a helper"));
        assert_eq!(read["severity"], "low", "{file}");
    }
    let mut defect = finding(
        "pay/premiacao_test.go",
        89,
        "high",
        "the test can never fail",
    );
    defect["kind"] = json!("defect");
    assert_eq!(signed_one(defect)["severity"], "high");

    let mut grep = finding("src/a.rs", 3, "high", "rule broken");
    grep["poc"] = json!("git grep -n unwrap src/");
    assert_eq!(signed_one(grep)["severity"], "medium");
}

#[test]
fn a_finding_without_a_kind_keeps_its_severity() {
    let read = signed_one(finding("src/a.rs", 3, "high", "xss"));
    assert_eq!(read["severity"], "high");
    assert!(read.get("capped").is_none());
    let mut low = finding("src/a.rs", 3, "low", "a name");
    low["kind"] = json!("style");
    assert!(signed_one(low).get("capped").is_none());
}

#[test]
fn a_kind_the_schema_does_not_name_is_refused() {
    let mut odd = finding("src/a.rs", 3, "low", "a name");
    odd["kind"] = json!("nit");
    let said = answer("claude", "devflow-spec", vec![odd]);
    let refused = read_signed(&said.to_string(), "claude", "devflow-spec").expect_err("refused");
    assert!(refused.contains("kind"), "{refused}");
}
