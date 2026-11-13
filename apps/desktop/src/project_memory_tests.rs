use super::*;

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("AGENTS.md"),
        "# Working in this repository\n\nRules.\n",
    )
    .expect("agents");
    std::fs::create_dir_all(dir.path().join("docs/adr")).expect("adr dir");
    std::fs::write(
        dir.path().join("docs/adr/0001-sync.md"),
        "# 1. Sync the person's workspace\n",
    )
    .expect("adr");
    dir
}

fn read(root: &Path, file: &str) -> String {
    std::fs::read_to_string(root.join(DIR).join(file)).unwrap_or_default()
}

#[test]
fn an_entry_carries_its_date_and_its_source() {
    let found = entry("- 2026-11-13 · AGENTS.md · Everything versioned is in English.")
        .expect("a bullet")
        .expect("an entry");
    assert_eq!(found.day, "2026-11-13");
    assert_eq!(found.source, "AGENTS.md");
    assert_eq!(found.text, "Everything versioned is in English.");

    assert!(entry("Each entry is a bullet.").is_none());
    assert!(entry("- Everything versioned is in English.")
        .expect("a bullet")
        .is_err());
    assert!(entry("- 13/11/2026 · AGENTS.md · English.")
        .expect("a bullet")
        .is_err());
    assert!(entry("- 2026-11-13 ·  · English.")
        .expect("a bullet")
        .is_err());
}

#[test]
fn the_first_wake_writes_three_files_whose_every_entry_has_a_date_and_a_source() {
    let dir = project();
    assert!(seed(dir.path(), "2026-11-13").expect("seeded"));
    for file in FILES {
        let text = read(dir.path(), file);
        assert!(text.starts_with("# "), "{file}: {text}");
        for line in text.lines().filter(|line| line.starts_with("- ")) {
            entry(line)
                .expect("a bullet")
                .unwrap_or_else(|why| panic!("{file}: {line}: {why}"));
        }
    }
    let facts = read(dir.path(), "facts.md");
    assert!(facts.contains("- 2026-11-13 · AGENTS.md · "), "{facts}");
    let decisions = read(dir.path(), "decisions.md");
    assert!(
        decisions.contains("- 2026-11-13 · docs/adr/0001-sync.md · 1. Sync the person's workspace"),
        "{decisions}"
    );
}

#[test]
fn memory_already_kept_is_never_written_over() {
    let dir = project();
    seed(dir.path(), "2026-11-13").expect("seeded");
    let mine = "# Facts\n\n- 2026-11-01 · PR #9 · The person's own line.\n";
    std::fs::write(dir.path().join(DIR).join("facts.md"), mine).expect("edited");
    assert!(!seed(dir.path(), "2026-12-01").expect("seeded again"));
    assert_eq!(read(dir.path(), "facts.md"), mine);
}

#[test]
fn a_project_without_memory_is_left_as_it_is() {
    let dir = project();
    assert_eq!(summary(dir.path()), None);
    assert!(!dir.path().join(".devpit").exists());
}

#[test]
fn the_summary_is_the_newest_entries_and_says_what_it_left_out() {
    let dir = project();
    seed(dir.path(), "2026-11-13").expect("seeded");
    let mut facts = "# Facts\n\n- not an entry at all\n".to_owned();
    for day in 1..=(SHOWN + 2) {
        facts.push_str(&format!("- 2026-10-{day:02} · card #{day} · Fact {day}.\n"));
    }
    std::fs::write(dir.path().join(DIR).join("facts.md"), facts).expect("facts");
    let summary = summary(dir.path()).expect("a summary");
    assert!(summary.contains("Fact 22."), "{summary}");
    assert!(!summary.contains("Fact 1."), "{summary}");
    assert!(summary.contains("2 older"), "{summary}");
    assert!(summary.contains("1 line"), "{summary}");
    assert!(
        summary.contains("1. Sync the person's workspace"),
        "{summary}"
    );
}

#[cfg(unix)]
#[test]
fn a_guide_or_an_adr_leading_out_of_the_project_is_not_read() {
    let outside = tempfile::tempdir().expect("outside");
    std::fs::write(outside.path().join("secret.md"), "# Not this project's\n").expect("secret");
    let dir = project();
    std::os::unix::fs::symlink(
        outside.path().join("secret.md"),
        dir.path().join("README.md"),
    )
    .expect("guide link");
    std::os::unix::fs::symlink(
        outside.path().join("secret.md"),
        dir.path().join("docs/adr/0002-outside.md"),
    )
    .expect("adr link");
    assert!(seed(dir.path(), "2026-11-13").expect("seeded"));
    let memory = read(dir.path(), "facts.md") + &read(dir.path(), "decisions.md");
    assert!(!memory.contains("Not this project's"), "{memory}");
    assert!(!memory.contains("README.md"), "{memory}");
    assert!(!memory.contains("0002-outside"), "{memory}");
    assert!(
        memory.contains("1. Sync the person's workspace"),
        "{memory}"
    );
}

#[test]
fn a_heading_is_looked_for_only_near_the_top_of_a_file() {
    let dir = project();
    let late = format!("{}# Too far down\n", "\n".repeat(HEAD_BYTES + 1));
    std::fs::write(dir.path().join("docs/adr/0003-long.md"), late).expect("long adr");
    seed(dir.path(), "2026-11-13").expect("seeded");
    let decisions = read(dir.path(), "decisions.md");
    assert!(!decisions.contains("Too far down"), "{decisions}");
    assert!(
        decisions.contains("· docs/adr/0003-long.md · docs/adr/0003-long.md"),
        "{decisions}"
    );
}

#[test]
fn an_approved_lesson_is_kept_with_its_source_and_date_and_a_discarded_one_leaves_no_trace() {
    let dir = project();
    let root = dir.path();
    seed(root, "2026-11-13").expect("seeded");
    propose(
        root,
        "2026-11-13",
        "#331",
        &[
            "Wait 3 s for the queue test.".to_owned(),
            "Run make fmt.".to_owned(),
        ],
    )
    .expect("proposed");
    let waiting = proposals(root);
    assert_eq!(waiting.len(), 2);
    assert_eq!(waiting[0].source, "#331");

    let edited = "Wait 3 s for the queue test under load.";
    decide(
        root,
        "2026-11-14",
        "#331",
        "Wait 3 s for the queue test.",
        Some(edited),
    )
    .expect("approved");
    decide(root, "2026-11-14", "#331", "Run make fmt.", None).expect("discarded");

    assert!(
        read(root, "lessons.md")
            .contains("- 2026-11-14 · #331 · Wait 3 s for the queue test under load.\n"),
        "{}",
        read(root, "lessons.md")
    );
    for file in ["lessons.md", "proposed.md", "facts.md", "decisions.md"] {
        assert!(!read(root, file).contains("make fmt"), "{file}");
    }
    assert!(proposals(root).is_empty());
    let kept = lessons(root);
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].text, edited);
    // Decided once: the same proposal is not there to decide again.
    assert!(decide(root, "2026-11-14", "#331", "Run make fmt.", None).is_err());
}

#[test]
fn review_rounds_and_repeated_findings_are_logged_per_card() {
    let dir = project();
    let root = dir.path();
    seed(root, "2026-11-13").expect("seeded");
    measure(root, "2026-11-13", "#331", 3, 2).expect("logged");
    measure(root, "2026-11-14", "#332", 1, 0).expect("logged");
    let lines: Vec<serde_json::Value> = read(root, "metrics.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).expect("a JSON line"))
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(
        lines[0],
        serde_json::json!({ "day": "2026-11-13", "card": "#331", "reviewRounds": 3, "repeatedFindings": 2 })
    );
}
