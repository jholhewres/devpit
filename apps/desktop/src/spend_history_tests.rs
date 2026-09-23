use super::*;

const NOW: i64 = 1_789_430_400; // 2026-09-15T00:00:00Z

fn record(key: &str, at: i64, session: &str, model: &str, output: u64) -> Record {
    Record {
        key: key.to_owned(),
        at,
        session_id: session.to_owned(),
        cwd: "/w/devpit".to_owned(),
        model: model.to_owned(),
        input: 1_000,
        output,
        cache_read: 3_000,
        cache_write_5m: 1_000,
        cache_write_1h: 0,
    }
}

fn installations() -> Vec<SpendInstallation> {
    vec![
        SpendInstallation {
            directory: "/h/.claude".to_owned(),
            label: ".claude".to_owned(),
            billed: true,
        },
        SpendInstallation {
            directory: "/h/.claude-glm".to_owned(),
            label: "glm".to_owned(),
            billed: false,
        },
    ]
}

fn summary(read: Vec<(usize, Record)>, days: u32) -> SpendHistory {
    summarize(
        read,
        installations(),
        NOW,
        days,
        |cwd| format!("project of {cwd}"),
        |session| {
            (session == "s-card").then(|| SpendCard {
                id: "card_1".to_owned(),
                title: "Wire the board".to_owned(),
            })
        },
    )
}

#[test]
fn a_message_counted_twice_is_counted_once_and_old_ones_fall_out_of_the_range() {
    let history = summary(
        vec![
            (0, record("m1:r1", NOW - 3_600, "s1", "claude-opus-5", 500)),
            (0, record("m1:r1", NOW - 3_600, "s1", "claude-opus-5", 500)),
            (
                0,
                record("m2:r2", NOW - 40 * DAY, "s1", "claude-opus-5", 500),
            ),
        ],
        7,
    );
    assert_eq!(history.turns, 1);
    assert_eq!(history.sessions, 1);
    assert_eq!(history.tokens.output, 500.0);
    assert_eq!(
        history.daily.len(),
        7,
        "one entry per day, empty days included"
    );
    assert_eq!(history.daily.last().expect("today").day, "2026-09-15");
    assert_eq!(history.active_days, 1);
    // 1000 fresh + 3000 read + 1000 written: three fifths served from the cache.
    assert!((history.cache_reuse - 0.6).abs() < 1e-9);
}

#[test]
fn models_and_projects_rank_by_cost_and_sessions_by_how_recently_they_ran() {
    let history = summary(
        vec![
            (
                0,
                record("a", NOW - 2 * DAY, "s-old", "claude-haiku-4-5", 100),
            ),
            (
                0,
                record("b", NOW - 3_600, "s-card", "claude-opus-5", 100_000),
            ),
            (0, record("c", NOW - 1_800, "s-card", "claude-opus-5", 100)),
        ],
        30,
    );
    let models: Vec<&str> = history.models.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(models, ["claude-opus-5", "claude-haiku-4-5"]);
    assert_eq!(history.projects[0].name, "project of /w/devpit");
    assert_eq!(history.projects[0].sessions, 2);
    let recent: Vec<&str> = history
        .recent
        .iter()
        .map(|one| one.session_id.as_str())
        .collect();
    assert_eq!(recent, ["s-card", "s-old"]);
    assert_eq!(history.recent[0].turns, 2);
    assert_eq!(
        history.recent[0]
            .card
            .as_ref()
            .map(|card| card.title.as_str()),
        Some("Wire the board")
    );
    assert_eq!(history.recent[1].card, None);
    let busiest = history
        .daily
        .iter()
        .rev()
        .find(|day| !day.by_model.is_empty())
        .expect("a day with work");
    assert_eq!(busiest.by_model[0].name, "claude-opus-5");
}

#[test]
fn an_installation_not_billed_by_anthropic_marks_its_dollars_estimated_and_unknown_models_are_unpriced(
) {
    let billed_only = summary(
        vec![(0, record("a", NOW - 60, "s1", "claude-opus-5", 10))],
        7,
    );
    assert!(!billed_only.estimated);

    let history = summary(
        vec![
            (1, record("b", NOW - 60, "s2", "claude-opus-5", 10)),
            (1, record("c", NOW - 50, "s2", "glm-4.6", 10)),
        ],
        7,
    );
    assert!(history.estimated);
    assert_eq!(history.unpriced_tokens, (1_000 + 10 + 3_000 + 1_000) as f64);
    assert_eq!(history.recent[0].installation, "glm");
}

#[test]
fn a_folder_no_project_claims_is_named_by_its_last_two_parts() {
    assert_eq!(short_path("/home/me/Workspace/asc/api"), "asc/api");
    assert_eq!(short_path("/tmp"), "tmp");
    assert_eq!(short_path(""), "");
}

#[test]
fn a_folder_contains_its_children_and_not_its_namesakes() {
    assert!(within("/w/devpit", "/w/devpit"));
    assert!(within("/w/devpit/web", "/w/devpit"));
    assert!(!within("/w/devpit-old", "/w/devpit"));
}

#[test]
fn an_installation_pointed_at_another_provider_is_not_billed() {
    let found = Found {
        directory: PathBuf::from("/h/.claude-glm"),
        said: Some("/h/.claude-glm".to_owned()),
        profiles: vec!["glm".to_owned()],
        default: false,
    };
    let profile = |url: &str| Declared {
        id: "p1".to_owned(),
        label: "glm".to_owned(),
        base: "claude".to_owned(),
        command: String::new(),
        args: Vec::new(),
        models: Vec::new(),
        env: vec![devpit_rpc::EnvVar {
            name: "ANTHROPIC_BASE_URL".to_owned(),
            value: url.to_owned(),
        }],
    };
    assert!(!billed(
        &found,
        &[profile("https://api.z.ai/api/anthropic")]
    ));
    assert!(billed(&found, &[profile("https://api.anthropic.com")]));
    assert!(billed(&found, &[]));
    assert_eq!(label_of(&found), "glm");
}
