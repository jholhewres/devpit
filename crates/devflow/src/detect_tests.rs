use super::*;

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, body) in files {
        let path = dir.path().join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, body).expect("write");
    }
    dir
}

fn cli(name: Agent, installed: bool, signed_in: Option<bool>) -> Cli {
    Cli {
        name,
        installed,
        signed_in,
    }
}

#[test]
fn a_makefile_target_is_found_with_its_line_and_settles_the_choice() {
    let dir = project(&[
        (
            "Makefile",
            "build:\n\tcargo build\n\ntest: build\n\tcargo test\nlint:\n\tcargo clippy\n",
        ),
        ("Cargo.toml", "[package]\nname = \"x\"\n"),
    ]);
    let found = detected(dir.path(), Machine::default());
    assert_eq!(found.test[0].command, "make test");
    assert_eq!(found.test[0].source, "Makefile:4");
    assert_eq!(found.test[1].command, "cargo test");
    assert_eq!(found.lint[0].source, "Makefile:6");
    assert!(open(&found).is_empty(), "{:?}", open(&found));
}

#[test]
fn two_test_commands_without_a_makefile_are_left_to_the_person() {
    let dir = project(&[
        (
            "package.json",
            "{\n  \"scripts\": {\n    \"test\": \"node t.js\"\n  }\n}\n",
        ),
        (
            "composer.json",
            "{\n  \"require-dev\": {\n    \"phpunit/phpunit\": \"^11\"\n  }\n}\n",
        ),
    ]);
    let found = detected(dir.path(), Machine::default());
    let said = open(&found);
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("npm test (package.json:3)"), "{said:?}");
    assert!(said[0].contains("composer.json:3"), "{said:?}");
}

#[test]
fn an_e2e_tool_nothing_runs_is_an_open_decision() {
    let dir = project(&[
        ("playwright.config.ts", "export default {}\n"),
        ("Makefile", "test:\n\tpytest\n"),
    ]);
    let found = detected(dir.path(), Machine::default());
    assert_eq!(found.e2e_tools[0].name, "playwright");
    assert!(open(&found)
        .iter()
        .any(|one| one.contains("playwright.config.ts") && one.contains("no command")));
}

#[test]
fn nothing_found_says_so() {
    let dir = project(&[]);
    let found = detected(dir.path(), Machine::default());
    assert_eq!(open(&found), ["no test command was found"]);
}

#[test]
fn a_cli_signed_out_and_a_project_opted_out_are_asked_about() {
    let dir = project(&[("Makefile", "test:\n\ttrue\n")]);
    let found = detected(
        dir.path(),
        Machine {
            clis: vec![
                cli(Agent::Claude, true, Some(true)),
                cli(Agent::Codex, true, Some(false)),
                cli(Agent::Gemini, true, None),
                cli(Agent::Cursor, false, None),
            ],
            decisions_key: true,
            decisions_opted_out: true,
        },
    );
    let said = open(&found);
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(said[0].starts_with("codex is installed but not signed in"));
    assert!(said[1].contains("opted out"));
}

#[test]
fn a_manifest_outside_the_project_is_not_read() {
    let outside = project(&[("Makefile", "test:\n\ttrue\n")]);
    let dir = project(&[]);
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path().join("Makefile"), dir.path().join("Makefile"))
        .expect("link");
    let found = detected(dir.path(), Machine::default());
    assert!(found.test.is_empty(), "{:?}", found.test);
}
