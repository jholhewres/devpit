//! The agents on this machine, as files.
//!
//! An agent is a markdown file with frontmatter in `~/.devpit/agents/`. That
//! is the whole format: writing a new one is adding a file, and it reaches the
//! next run without a restart, a registry or a recompile.
//!
//! A set is seeded on first use from whatever the person already has
//! installed, so the product starts with agents rather than with an empty
//! directory and an invitation to write one.

use std::path::{Path, PathBuf};

/// One agent, as the CLI wants to be told about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub tools: Option<Vec<String>>,
    pub prompt: String,
}

/// A file that did not load, and why.
///
/// Returned alongside the agents rather than instead of them: one malformed
/// file must not hide the twenty that are fine.
#[derive(Debug, Clone, PartialEq)]
pub struct Rejected {
    pub file: String,
    pub reason: String,
}

/// Everything in the directory, and everything that would not load.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Catalogue {
    pub agents: Vec<Agent>,
    pub rejected: Vec<Rejected>,
}

/// Reads the directory. A directory that is not there yet is empty, not an
/// error: the first run has not seeded it.
pub fn read(dir: &Path) -> Catalogue {
    let mut catalogue = Catalogue::default();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return catalogue;
    };

    let mut paths: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    paths.sort();

    for path in paths {
        let file = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match std::fs::read_to_string(&path) {
            Ok(text) => match parse(&text) {
                Ok(agent) => catalogue.agents.push(agent),
                Err(reason) => catalogue.rejected.push(Rejected { file, reason }),
            },
            Err(err) => catalogue.rejected.push(Rejected {
                file,
                reason: err.to_string(),
            }),
        }
    }

    catalogue
}

/// Splits frontmatter from body and reads the fields we use.
///
/// Unknown keys are ignored on purpose: these files come from elsewhere and
/// carry fields this product has no opinion about. Refusing them would make
/// the format ours instead of shared.
fn parse(text: &str) -> Result<Agent, String> {
    let rest = text
        .strip_prefix("---\n")
        .ok_or("no frontmatter: the file has to open with ---")?;
    let (front, body) = rest
        .split_once("\n---")
        .ok_or("the frontmatter is never closed with ---")?;

    let mut name = None;
    let mut description = String::new();
    let mut model = None;
    let mut tools = None;

    for line in front.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "name" => name = Some(value.to_owned()),
            "description" => description = value.to_owned(),
            "model" => model = Some(value.to_owned()),
            "tools" => {
                tools = Some(
                    value
                        .trim_matches(['[', ']'])
                        .split(',')
                        .map(|t| t.trim().to_owned())
                        .filter(|t| !t.is_empty())
                        .collect(),
                )
            }
            _ => {}
        }
    }

    Ok(Agent {
        name: name.ok_or("no `name` in the frontmatter")?,
        description,
        model,
        tools,
        prompt: body.trim_start_matches('-').trim().to_owned(),
    })
}

/// Copies agents from `source` into `dir`, never overwriting.
///
/// Not overwriting is the whole point: the seed is a starting set, and an
/// agent the person edited is theirs. Returns how many were new.
///
/// A file is copied only if it reads as an agent. Those directories also hold
/// `AGENTS.md` and README files that share the extension and are not agents;
/// copying them would put a permanent error in the catalogue on first run.
pub fn seed(dir: &Path, source: &Path) -> std::io::Result<usize> {
    let Ok(entries) = std::fs::read_dir(source) else {
        return Ok(0);
    };
    std::fs::create_dir_all(dir)?;

    let mut copied = 0;
    for entry in entries.filter_map(Result::ok) {
        let from = entry.path();
        if from.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let Some(name) = from.file_name() else {
            continue;
        };
        let to = dir.join(name);
        if to.exists() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&from) else {
            continue;
        };
        if parse(&text).is_err() {
            continue;
        }
        std::fs::copy(&from, &to)?;
        copied += 1;
    }
    Ok(copied)
}

/// The catalogue as the `--agents` argument wants it.
pub fn as_argument(agents: &[Agent]) -> String {
    let entries: Vec<String> = agents
        .iter()
        .map(|agent| {
            let mut fields = vec![
                format!("\"description\":{}", quote(&agent.description)),
                format!("\"prompt\":{}", quote(&agent.prompt)),
            ];
            if let Some(model) = &agent.model {
                fields.push(format!("\"model\":{}", quote(model)));
            }
            if let Some(tools) = &agent.tools {
                let list: Vec<String> = tools.iter().map(|t| quote(t)).collect();
                fields.push(format!("\"tools\":[{}]", list.join(",")));
            }
            format!("{}:{{{}}}", quote(&agent.name), fields.join(","))
        })
        .collect();
    format!("{{{}}}", entries.join(","))
}

fn quote(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

#[cfg(test)]
mod tests {
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

    /// One broken file must not hide the ones that are fine.
    #[test]
    fn a_broken_file_is_named_and_the_rest_still_load() {
        let dir = dir_with(&[
            ("good.md", REVIEWER),
            ("broken.md", "no frontmatter here\n"),
            ("nameless.md", "---\ndescription: x\n---\nbody\n"),
        ]);
        let catalogue = read(dir.path());

        assert_eq!(catalogue.agents.len(), 1, "the good one did not survive");
        assert_eq!(catalogue.rejected.len(), 2);
        let files: Vec<&str> = catalogue.rejected.iter().map(|r| r.file.as_str()).collect();
        assert!(files.contains(&"broken.md") && files.contains(&"nameless.md"));
        assert!(catalogue.rejected.iter().any(|r| r.reason.contains("name")));
    }

    /// A directory that does not exist yet is empty, not an error.
    #[test]
    fn a_missing_directory_is_empty_not_a_failure() {
        assert_eq!(read(Path::new("/nonexistent/agents")), Catalogue::default());
    }

    /// Those directories hold documentation with the same extension. Copying
    /// it would put a permanent error in the catalogue on first run.
    #[test]
    fn the_seed_skips_a_markdown_file_that_is_not_an_agent() {
        let source = dir_with(&[
            ("reviewer.md", REVIEWER),
            ("AGENTS.md", "# How to work in this repository\n"),
        ]);
        let target = tempfile::tempdir().expect("tempdir");

        assert_eq!(seed(target.path(), source.path()).expect("seed"), 1);
        assert!(read(target.path()).rejected.is_empty());
    }

    /// The seed runs on every start. Overwriting on the second one would undo
    /// whatever the person changed.
    #[test]
    fn seeding_twice_never_overwrites_an_edit() {
        let source = dir_with(&[("reviewer.md", REVIEWER)]);
        let target = tempfile::tempdir().expect("tempdir");

        assert_eq!(seed(target.path(), source.path()).expect("seed"), 1);

        let mine = target.path().join("reviewer.md");
        std::fs::write(&mine, "---\nname: mine\n---\nedited\n").expect("edit");

        assert_eq!(seed(target.path(), source.path()).expect("reseed"), 0);
        assert_eq!(
            std::fs::read_to_string(&mine).expect("read"),
            "---\nname: mine\n---\nedited\n",
            "the seed overwrote an edited agent"
        );
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
        }];

        let argument = as_argument(&agents);
        let parsed: serde_json::Value = serde_json::from_str(&argument).expect("valid JSON");
        assert_eq!(parsed["reviewer"]["description"], "says \"no\"");
        assert_eq!(parsed["reviewer"]["prompt"], "line one\nline \"two\"");
        assert_eq!(parsed["reviewer"]["tools"][0], "Read");
    }

    /// The set that ships is read from files someone else wrote, so the real
    /// ones are the test: a field they use and we do not must not reject them.
    #[test]
    fn the_agents_installed_on_this_machine_load() {
        let sources = crate::seed_sources();
        if sources.is_empty() {
            eprintln!("skipped: no agent set installed to seed from");
            return;
        }

        let target = tempfile::tempdir().expect("tempdir");
        let mut copied = 0;
        for source in &sources {
            copied += seed(target.path(), source).expect("seed");
        }

        let catalogue = read(target.path());
        assert!(copied > 0, "seeded nothing from {} sources", sources.len());
        assert_eq!(catalogue.agents.len(), copied, "an agent did not load back");
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
}
