use super::*;

const REVIEWER: &str = "---\nname: code-reviewer\ndescription: Reviews code\nmodel: claude-opus-4-6\ntools: [Read, Grep]\n---\n\nYou review code.\n";

fn dir_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (name, contents) in files {
        std::fs::write(dir.path().join(name), contents).expect("write");
    }
    dir
}

#[test]
fn an_agent_is_a_markdown_file_with_frontmatter() {
    let dir = dir_with(&[("reviewer.md", REVIEWER)]);
    let catalogue = read(dir.path());

    assert_eq!(catalogue.agents.len(), 1);
    let agent = &catalogue.agents[0];
    assert_eq!(agent.name, "code-reviewer");
    assert_eq!(agent.model.as_deref(), Some("claude-opus-4-6"));
    assert_eq!(
        agent.tools.as_deref(),
        Some(["Read".to_owned(), "Grep".to_owned()].as_slice())
    );
    assert_eq!(agent.prompt, "You review code.");
}

/// One broken file must not hide the ones that are fine. Broken means it
/// tried to be an agent and failed; a README that shares the extension
/// never claimed to be one and is not reported.
#[test]
fn a_broken_file_is_named_and_the_rest_still_load() {
    let dir = dir_with(&[
        ("good.md", REVIEWER),
        ("AGENTS.md", "# How to work in this repository\n"),
        ("nameless.md", "---\ndescription: x\n---\nbody\n"),
    ]);
    let catalogue = read(dir.path());

    assert_eq!(catalogue.agents.len(), 1, "the good one did not survive");
    assert_eq!(catalogue.rejected.len(), 1, "{:?}", catalogue.rejected);
    assert_eq!(catalogue.rejected[0].file, "nameless.md");
    assert!(catalogue.rejected[0].reason.contains("name"));
}

/// A directory that does not exist yet is empty, not an error.
#[test]
fn a_missing_directory_is_empty_not_a_failure() {
    assert_eq!(read(Path::new("/nonexistent/agents")), Catalogue::default());
}

/// Every agent is read where it stands. Nothing is copied into devpit,
/// so there is never a second, stale answer to what an agent does.
#[test]
fn nothing_is_copied_and_each_agent_says_where_it_came_from() {
    let home = tempfile::tempdir().expect("tempdir");
    let mine = home.path().join(".devpit/agents");
    let plugin = home.path().join("plugins/omc/agents");
    std::fs::create_dir_all(&mine).expect("create");
    std::fs::create_dir_all(&plugin).expect("create");
    std::fs::write(
        mine.join("mine.md"),
        REVIEWER.replace("code-reviewer", "mine"),
    )
    .expect("write");
    std::fs::write(
        plugin.join("architect.md"),
        REVIEWER.replace("code-reviewer", "architect"),
    )
    .expect("write");

    let found = read_all(&[mine.clone(), plugin.clone()]);
    let sources: Vec<(String, String)> = found
        .agents
        .iter()
        .map(|agent| (agent.name.clone(), agent.source.clone()))
        .collect();
    assert_eq!(
        sources,
        vec![
            ("architect".to_owned(), "omc".to_owned()),
            ("mine".to_owned(), "yours".to_owned()),
        ]
    );

    // The source directories are still the only ones that exist.
    assert_eq!(std::fs::read_dir(&mine).expect("read").count(), 1);
    assert_eq!(std::fs::read_dir(&plugin).expect("read").count(), 1);
}

/// The person's own set is passed first, so it shadows a plugin's rather
/// than the other way round.
#[test]
fn your_own_agent_wins_a_name_it_shares_with_a_plugins() {
    let home = tempfile::tempdir().expect("tempdir");
    let mine = home.path().join(".devpit/agents");
    let plugin = home.path().join("plugins/omc/agents");
    std::fs::create_dir_all(&mine).expect("create");
    std::fs::create_dir_all(&plugin).expect("create");
    std::fs::write(mine.join("a.md"), REVIEWER).expect("write");
    std::fs::write(plugin.join("a.md"), REVIEWER).expect("write");

    let found = read_all(&[mine, plugin]);
    assert_eq!(found.agents.len(), 1);
    assert_eq!(found.agents[0].source, "yours");
}

/// A file dropped into the directory is an agent. No registration step.
#[test]
fn an_agent_written_by_hand_shows_up() {
    let dir = dir_with(&[("reviewer.md", REVIEWER)]);
    std::fs::write(
        dir.path().join("mine.md"),
        "---\nname: mine\ndescription: my own\n---\nDo it my way.\n",
    )
    .expect("write");

    let names: Vec<String> = read(dir.path())
        .agents
        .into_iter()
        .map(|a| a.name)
        .collect();
    assert!(names.contains(&"mine".to_owned()));
}

/// The argument has to be JSON the CLI accepts, and a prompt with quotes
/// or newlines in it must not break the line.
#[test]
fn the_argument_is_valid_json() {
    let agents = vec![Agent {
        name: "reviewer".to_owned(),
        description: "says \"no\"".to_owned(),
        model: None,
        tools: Some(vec!["Read".to_owned()]),
        prompt: "line one\nline \"two\"".to_owned(),
        source: "yours".to_owned(),
    }];

    let argument = as_argument(&agents);
    let parsed: serde_json::Value = serde_json::from_str(&argument).expect("valid JSON");
    assert_eq!(parsed["reviewer"]["description"], "says \"no\"");
    assert_eq!(parsed["reviewer"]["prompt"], "line one\nline \"two\"");
    assert_eq!(parsed["reviewer"]["tools"][0], "Read");
}

/// The set on this machine is files someone else wrote, so the real ones
/// are the test: a field they use and we do not must not reject them.
#[test]
fn the_agents_installed_on_this_machine_load() {
    let sources = crate::seed_sources();
    if sources.is_empty() {
        eprintln!("skipped: no agent set installed on this machine");
        return;
    }

    let catalogue = read_all(&sources);
    assert!(
        !catalogue.agents.is_empty(),
        "read nothing from {} sources",
        sources.len()
    );
    assert!(
        catalogue.rejected.is_empty(),
        "files that would not load: {:?}",
        catalogue.rejected
    );
}

/// The argument is only right if the CLI takes it.
#[test]
fn the_installed_cli_accepts_the_argument_we_build() {
    if !crate::available() {
        eprintln!("skipped: the agent CLI is not on PATH");
        return;
    }
    let agents = vec![Agent {
        name: "probe".to_owned(),
        description: "a probe".to_owned(),
        model: None,
        tools: Some(vec![]),
        prompt: "Answer with the single word PONG.".to_owned(),
        source: "yours".to_owned(),
    }];

    // --help parses the whole line and exits without spending anything.
    let output = std::process::Command::new(crate::PROGRAM)
        .args(["-p", "--agents", &as_argument(&agents), "--help"])
        .output()
        .expect("run the CLI");
    assert!(
        output.status.success(),
        "the CLI refused the --agents argument: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}
