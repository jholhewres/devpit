//! The skills installed on this machine, and what one of them says.
//!
//! Out of `workspace.rs` because they are different questions: that file
//! answers "how much room is this taking", four fixed rows and a total, and
//! this one answers "what does this machine know how to do".
//!
//! Both the list and the body are read from disk every time they are asked
//! for. A skill arrives by installing a plugin, which happens in a terminal
//! beside this window — a catalogue cached at startup is a catalogue that is
//! wrong by the time anyone looks at it.

use std::path::{Path, PathBuf};

use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Skill {
    pub name: String,
    /// Where it came from — `omc`, `claude`, `yours`.
    pub source: String,
    /// The `SKILL.md` itself, so Open and Reveal have something to hand over.
    pub path: String,
    /// The author's own one line, from the frontmatter.
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Skills {
    pub skills: Vec<Skill>,
    /// Why nothing could be read, when nothing could. Present and non-empty
    /// means the panel says this instead of looking empty.
    pub problem: Option<String>,
    /// The CLI configuration directory these came from.
    ///
    /// On screen because it is not always `~/.claude`, and a panel listing
    /// another installation's skills is indistinguishable from a panel
    /// listing this one's — see `cli_config`.
    pub directory: String,
}

/// One skill, with the whole of its `SKILL.md`.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SkillDoc {
    pub name: String,
    pub path: String,
    /// The file as written, Markdown and all, with the frontmatter removed.
    pub body: String,
}

/// Read no more of a `SKILL.md` than this.
///
/// They are prose; the largest on the machine this was written on is 44 KB.
/// A skill directory is somebody else's file though, and a read with no
/// ceiling is a window that can be handed a gigabyte to draw.
const MOST_BYTES: u64 = 512 * 1024;

/// The frontmatter block, and the rest of the file.
///
/// Returned as a pair rather than parsed into fields: two readers want
/// different halves, and the split is the only part they agree on.
fn split_frontmatter(text: &str) -> (&str, &str) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return ("", text);
    };
    match rest.split_once("\n---") {
        // The rest of the closing `---` line, then the body. Trimming dashes
        // instead ate the marker of a list that began right under it.
        Some((front, after)) => (front, after.split_once('\n').map_or("", |(_, body)| body)),
        None => ("", text),
    }
}

/// The author's own description, from the frontmatter.
///
/// Every one of the 244 `SKILL.md` files on the machine this was written on
/// carries a `description:`, and 14 of them write it as a `|` block over
/// several lines. The first line of the value is what a one-line row can
/// hold, so that is what this takes.
fn described_in(front: &str) -> Option<String> {
    let mut lines = front.lines();
    let rest = lines.find_map(|line| line.strip_prefix("description:"))?;
    let said = rest.trim();
    // `|` and `>` say the value is the indented block below, not this line.
    if !said.is_empty() && !matches!(said, "|" | ">" | "|-" | ">-" | "|+" | ">+") {
        return Some(said.trim_matches(['"', '\'']).to_owned());
    }
    // Only the indented lines are the block. An empty `|` followed by the
    // next key would otherwise describe the skill as `name: x`.
    lines
        .take_while(|line| line.starts_with([' ', '\t']) || line.trim().is_empty())
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

/// The line under a skill's name.
///
/// The frontmatter first, because that is the field written to be exactly
/// this. Before, it was the first line of prose in the body — which for the
/// skills on this machine is the opening sentence of the instructions, often
/// two hundred characters of it, in a row one line tall.
pub fn description_of(text: &str) -> String {
    let (front, body) = split_frontmatter(text);
    if let Some(said) = described_in(front) {
        return said;
    }
    body.lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with("---"))
        .unwrap_or_default()
        .to_owned()
}

/// The `SKILL.md` of every skill directory under `dir`, in the order read.
fn skills_in(dir: &Path, source: &str, into: &mut Vec<Skill>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let file = entry.path().join("SKILL.md");
        if !file.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if into.iter().any(|had: &Skill| had.name == name) {
            continue;
        }
        into.push(Skill {
            description: std::fs::read_to_string(&file)
                .map(|text| description_of(&text))
                .unwrap_or_default(),
            path: file.display().to_string(),
            name,
            source: source.to_owned(),
        });
    }
}

/// The skill directories of the installation asked for.
fn sources_of(directory: Option<&str>) -> Result<(String, Vec<PathBuf>), RpcError> {
    let chosen = crate::installations::chosen(directory)?;
    let home = crate::installations::home()?;
    Ok((
        chosen.directory.display().to_string(),
        devpit_agentcli::skills::sources_in(&chosen.directory, &home),
    ))
}

/// `skills.list` — the skills of one installation of the CLI, the default
/// profile's when none is named.
#[tauri::command]
#[specta::specta]
pub fn skills_list(directory: Option<String>) -> Result<Skills, RpcError> {
    let (directory, sources) = sources_of(directory.as_deref())?;
    if sources.is_empty() {
        return Ok(Skills {
            skills: Vec::new(),
            problem: Some("no skills directory on this machine".to_owned()),
            directory,
        });
    }

    let mut skills = Vec::new();
    for dir in &sources {
        skills_in(dir, &devpit_agentcli::source_of(dir), &mut skills);
    }
    skills.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(Skills {
        problem: skills
            .is_empty()
            .then(|| "the skills directories are there but hold nothing".to_owned()),
        skills,
        directory,
    })
}

/// Where a named skill's `SKILL.md` is, or nothing.
///
/// By name and never by a path from the screen: this process reads whatever it
/// is handed, so the name is looked up in the directories that are already
/// trusted rather than resolved against one of them.
fn found(name: &str, sources: Vec<PathBuf>) -> Option<PathBuf> {
    sources
        .into_iter()
        .map(|dir| dir.join(name).join("SKILL.md"))
        .find(|file| file.is_file())
}

/// `skills.read` — the whole of one skill's `SKILL.md`.
#[tauri::command]
#[specta::specta]
pub fn skills_read(name: String, directory: Option<String>) -> Result<SkillDoc, RpcError> {
    // A name with a separator in it is a path pretending to be a name.
    if name.contains(['/', '\\']) || name.starts_with('.') {
        return Err(RpcError::new(ErrorCode::Forbidden, "that is not a skill"));
    }
    let file = found(&name, sources_of(directory.as_deref())?.1)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, format!("no skill called {name}")))?;

    let bytes = std::fs::metadata(&file)
        .map(|meta| meta.len())
        .unwrap_or_default();
    if bytes > MOST_BYTES {
        return Err(RpcError::internal(format!(
            "{name} is {:.1} MB — past the {} KB this reads",
            bytes as f64 / 1_048_576.0,
            MOST_BYTES / 1024
        )));
    }

    let text = std::fs::read_to_string(&file).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(SkillDoc {
        body: split_frontmatter(&text).1.trim_start().to_owned(),
        path: file.display().to_string(),
        name,
    })
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
