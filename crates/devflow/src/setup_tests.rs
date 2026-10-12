use super::*;
use crate::config::{Check, Report};

fn with_checks(runs: &[(&str, &str)]) -> Config {
    let mut config = Config::default();
    config.stages.execution.checks = runs
        .iter()
        .map(|(name, run)| Check {
            name: (*name).into(),
            run: (*run).into(),
            report: Report::Nothing,
            evidence: Vec::new(),
        })
        .collect();
    config
}

fn changes_of(done: &[FileChange]) -> Vec<(&str, &str)> {
    done.iter()
        .map(|one| (one.path.as_str(), one.change))
        .collect()
}

#[test]
fn each_command_moves_into_a_script_that_says_where_it_came_from() {
    let config = with_checks(&[("unit", "make test"), ("lint", &script_run("lint"))]);
    let sources = [("make test".to_owned(), "Makefile:4".to_owned())];
    let (config, scripts) = scripted(config, &sources);
    assert_eq!(
        config.stages.execution.checks[0].run,
        "sh .devpit/devflow/scripts/check-unit.sh"
    );
    // Already a script of its own: left as it is.
    assert_eq!(scripts.len(), 1);
    let text = script_text(&scripts[0]);
    assert!(text.starts_with("#!/bin/sh\n"), "{text}");
    assert!(text.contains("found in Makefile:4"), "{text}");
    assert!(text.ends_with("\nmake test\n"), "{text}");
}

#[test]
fn a_setup_writes_the_folder_and_nothing_before_it_is_asked() {
    let root = tempfile::tempdir().expect("tempdir");
    let (config, scripts) = scripted(with_checks(&[("unit", "make test")]), &[]);
    let files = planned(&crate::render::render(&config), &scripts, None);

    let previewed = preview(root.path(), &files);
    assert!(previewed.iter().all(|one| one.change == "create"));
    assert!(!root.path().join(".devpit").exists());

    let done = write(root.path(), &files).expect("written");
    assert_eq!(
        changes_of(&done),
        [
            (".devpit/devflow/devflow.yaml", "create"),
            (".devpit/devflow/.gitignore", "create"),
            (".devpit/devflow/scripts/check-unit.sh", "create"),
            (".devpit/devflow/reviewers/devflow-spec.md", "create"),
            (".devpit/devflow/reviewers/devflow-standards.md", "create"),
            (".devpit/devflow/reviewers/devflow-security.md", "create"),
            (".devpit/devflow/devflow-findings.schema.json", "create"),
        ]
    );
    let ignore =
        std::fs::read_to_string(root.path().join(".devpit/devflow/.gitignore")).expect("gitignore");
    assert!(ignore.lines().any(|line| line == "runs/"), "{ignore}");
    let persona = std::fs::read_to_string(
        root.path()
            .join(".devpit/devflow/reviewers/devflow-security.md"),
    )
    .expect("persona");
    assert!(
        persona.starts_with("---\nname: devflow-security"),
        "{persona}"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(root.path().join(".devpit/devflow/scripts/check-unit.sh"))
            .expect("script")
            .permissions()
            .mode();
        assert_eq!(mode & 0o111, 0o111);
    }

    // Again: nothing to do, and the person's own persona and ignores stay.
    std::fs::write(
        root.path()
            .join(".devpit/devflow/reviewers/devflow-spec.md"),
        "ours\n",
    )
    .expect("edit");
    std::fs::write(root.path().join(".devpit/devflow/.gitignore"), "*.log").expect("edit");
    let again = write(root.path(), &files).expect("again");
    let said = changes_of(&again);
    assert_eq!(said[0], (".devpit/devflow/devflow.yaml", "same"));
    assert_eq!(said[1], (".devpit/devflow/.gitignore", "append"));
    assert_eq!(
        said[3],
        (".devpit/devflow/reviewers/devflow-spec.md", "kept")
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join(".devpit/devflow/.gitignore")).expect("read"),
        "*.log\nruns/\n"
    );
}

#[cfg(unix)]
#[test]
fn a_folder_that_leads_outside_the_project_is_refused() {
    let root = tempfile::tempdir().expect("root");
    let elsewhere = tempfile::tempdir().expect("elsewhere");
    std::os::unix::fs::symlink(elsewhere.path(), root.path().join(".devpit")).expect("link");
    let files = config_only("version: 1\n");
    assert!(write(root.path(), &files).is_err());
    assert!(std::fs::read_dir(elsewhere.path())
        .expect("read")
        .next()
        .is_none());
}
