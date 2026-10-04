use super::*;

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in files {
        std::fs::write(dir.path().join(name), body).expect("write");
    }
    dir
}

#[test]
fn a_makefile_test_target_comes_first() {
    let dir = project(&[
        (
            "Makefile",
            "build:\n\tcargo build\ntest: build\n\tcargo test\n",
        ),
        ("Cargo.toml", ""),
    ]);
    assert_eq!(detected(dir.path()).command.as_deref(), Some("make test"));
}

#[test]
fn vitest_writes_its_report_where_the_step_reads_it() {
    let dir = project(&[
        (
            "package.json",
            r#"{"scripts":{"test":"vitest"},"devDependencies":{"vitest":"^4"}}"#,
        ),
        ("pnpm-lock.yaml", ""),
    ]);
    let found = detected(dir.path());
    let command = found.command.expect("a command");
    assert!(command.starts_with("pnpm exec vitest run"), "{command}");
    assert!(command.contains("$DEVPIT_REPORT_DIR"), "{command}");
    assert_eq!(found.from.as_deref(), Some("package.json"));
}

#[test]
fn a_plain_script_runs_through_its_package_manager() {
    let dir = project(&[
        ("package.json", r#"{"scripts":{"test":"node t.js"}}"#),
        ("yarn.lock", ""),
    ]);
    assert_eq!(detected(dir.path()).command.as_deref(), Some("yarn test"));
    let npm = project(&[("package.json", r#"{"scripts":{"test":"node t.js"}}"#)]);
    assert_eq!(detected(npm.path()).command.as_deref(), Some("npm test"));
}

#[test]
fn cargo_then_nothing() {
    assert_eq!(
        detected(project(&[("Cargo.toml", "")]).path())
            .command
            .as_deref(),
        Some("cargo test")
    );
    assert_eq!(
        detected(project(&[]).path()),
        TestCommand {
            command: None,
            from: None
        }
    );
}
