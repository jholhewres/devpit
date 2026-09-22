//! The agents on this machine, as files.
//!
//! An agent is a markdown file with frontmatter. That is the whole format:
//! writing a new one is adding a file, and it reaches the next run without a
//! restart, a registry or a recompile.
//!
//! **Referenced, never copied.** The agents on this machine belong to whoever
//! installed them; a copy taken into `~/.devpit` would be stale the week
//! after, and there would be two answers to "what does architect do". Every
//! directory is read where it stands, and each agent says where it came
//! from.

use std::path::{Path, PathBuf};

/// One agent, as the CLI wants to be told about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Agent {
    pub name: String,
    pub description: String,
    pub model: Option<String>,
    pub tools: Option<Vec<String>>,
    pub prompt: String,
    /// Where it came from, for the screen: `architect · omc`. Two agents with
    /// the same name from different places are told apart by this and nothing
    /// else.
    pub source: String,
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
            // A markdown file with no frontmatter never claimed to be an
            // agent. These directories also hold AGENTS.md and READMEs, and
            // reporting those as broken puts a permanent error on screen for
            // files nobody wrote as agents.
            Ok(text) if !text.starts_with("---\n") => {}
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
        source: String::new(),
    })
}

/// Every agent across every directory, each tagged with where it came from.
///
/// First directory wins a name clash: the caller passes its own set first, so
/// an agent the person wrote shadows one a plugin installed rather than the
/// other way round.
pub fn read_all(dirs: &[PathBuf]) -> Catalogue {
    let mut all = Catalogue::default();
    for dir in dirs {
        let source = source_of(dir);
        let mut here = read(dir);
        for agent in &mut here.agents {
            agent.source = source.clone();
        }
        here.agents
            .retain(|agent| !all.agents.iter().any(|had| had.name == agent.name));
        all.agents.extend(here.agents);
        all.rejected.extend(here.rejected);
    }
    all.agents.sort_by(|a, b| a.name.cmp(&b.name));
    all
}

/// What to call the place an agent came from.
///
/// The directory is always `.../<something>/agents`, so the name is the
/// segment above it — the plugin's own name, which is what a person recognises.
pub fn source_of(dir: &Path) -> String {
    let named = dir
        .parent()
        .and_then(|parent| parent.file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    match named.as_str() {
        /* Both homes: a development build keeps `.devpit-dev` beside the
        installed `.devpit` (`devpit_core::DEV_ROOT`), and its own agents are
        just as much yours. Written out rather than imported because this
        crate does not depend on core. */
        ".devpit" | ".devpit-dev" => "yours".to_owned(),
        "" => "unknown".to_owned(),
        // `~/.claude/agents` would read as ".claude", which is a path showing
        // through into the screen — and so would `~/.claude-claudin`, which is
        // what a second installation of the same CLI is actually called.
        other if other.starts_with(".claude") => "claude".to_owned(),
        other => other.to_owned(),
    }
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
#[path = "catalogue_tests.rs"]
mod tests;

/// Apart from `catalogue_tests.rs` because it is a different question: that
/// file is what loads, and this is what the loaded thing is called.
#[cfg(test)]
#[path = "source_name_tests.rs"]
mod source_name;
