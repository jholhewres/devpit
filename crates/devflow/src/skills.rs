//! The flow's skills — one per stage, and one for its setup — as a project
//! resolves them: its own `.devpit/devflow/skills/<name>/SKILL.md`, else the
//! person's under devpit's home, else the one built into the app from
//! flow/skills/. A layer replaces a skill by its name; none adds one.
//!
//! Each is described the way the MCP Skills extension (SEP-2640) does: a
//! `skill://` URI, its frontmatter, and its file with digest and size.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::folder;

/// A project's own skills, relative to its root.
pub const DIR: &str = ".devpit/devflow/skills";

/// The most of a SKILL.md read, checked before reading: far past what a
/// context should take whole.
pub const MOST_BYTES: u64 = 128 * 1024;

/// Each skill's name, its stage (none for the setup's), and its text as this
/// build ships it.
const BUILT_IN: [(&str, Option<&str>, &str); 7] = [
    (
        "devflow-briefing",
        Some("briefing"),
        include_str!("../../../flow/skills/devflow-briefing/SKILL.md"),
    ),
    (
        "devflow-context",
        Some("context"),
        include_str!("../../../flow/skills/devflow-context/SKILL.md"),
    ),
    (
        "devflow-plan",
        Some("plan"),
        include_str!("../../../flow/skills/devflow-plan/SKILL.md"),
    ),
    (
        "devflow-execution",
        Some("execution"),
        include_str!("../../../flow/skills/devflow-execution/SKILL.md"),
    ),
    (
        "devflow-review",
        Some("review"),
        include_str!("../../../flow/skills/devflow-review/SKILL.md"),
    ),
    (
        "devflow-completion",
        Some("completion"),
        include_str!("../../../flow/skills/devflow-completion/SKILL.md"),
    ),
    (
        "devflow-setup",
        None,
        include_str!("../../../flow/skills/devflow-setup/SKILL.md"),
    ),
];

/// Where a skill was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Project,
    Personal,
    BuiltIn,
}

impl Layer {
    pub fn word(self) -> &'static str {
        match self {
            Layer::Project => "project",
            Layer::Personal => "personal",
            Layer::BuiltIn => "built-in",
        }
    }
}

/// One skill, resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Skill {
    pub name: &'static str,
    pub stage: Option<&'static str>,
    pub layer: Layer,
    /// The file it was read from; none when built in.
    pub file: Option<PathBuf>,
    pub text: String,
    /// The SKILL.md frontmatter as JSON.
    pub frontmatter: Value,
}

/// Every skill devpit has, in the order the work reaches them.
pub fn names() -> impl Iterator<Item = &'static str> {
    BUILT_IN.iter().map(|(name, ..)| *name)
}

/// What an active project is offered: the skills of the stages switched on,
/// and the setup's.
pub fn offered(stages_on: &[&str]) -> Vec<&'static str> {
    BUILT_IN
        .iter()
        .filter(|(_, stage, _)| stage.is_none_or(|stage| stages_on.contains(&stage)))
        .map(|(name, ..)| *name)
        .collect()
}

/// The resource URI of a skill's SKILL.md: `devflow` the prefix, the name the
/// last segment, as the extension requires.
pub fn uri(name: &str) -> String {
    format!("skill://devflow/{name}/SKILL.md")
}

/// The skill a name or its URI stands for, when it is one of devpit's.
pub fn named(name_or_uri: &str) -> Option<&'static str> {
    names().find(|name| *name == name_or_uri || uri(name) == name_or_uri)
}

/// `name` from the first layer that has it: the project's, the person's
/// (`personal`), the built-in one. A file that is there but cannot be used —
/// outside its folder, too large, not UTF-8, another name in its frontmatter
/// — is refused, never passed over for the next layer.
pub fn resolve(project: &Path, personal: &Path, name: &str) -> Result<Skill, String> {
    let (name, stage, built_in) = BUILT_IN
        .iter()
        .find(|(one, ..)| *one == name)
        .copied()
        .ok_or_else(|| {
            format!(
                "`{name}` is not a Devflow skill: {}",
                names().collect::<Vec<_>>().join(", ")
            )
        })?;
    let layers = [
        (Layer::Project, project, format!("{DIR}/{name}/SKILL.md")),
        (Layer::Personal, personal, format!("{name}/SKILL.md")),
    ];
    for (layer, base, relative) in layers {
        let refused = |why: String| format!("the {} {relative}: {why}", layer.word());
        let Some((file, text)) = read_in(base, &relative).map_err(refused)? else {
            continue;
        };
        let frontmatter = frontmatter(&text, name).map_err(refused)?;
        return Ok(Skill {
            name,
            stage,
            layer,
            file: Some(file),
            text,
            frontmatter,
        });
    }
    Ok(Skill {
        name,
        stage,
        layer: Layer::BuiltIn,
        file: None,
        text: built_in.to_owned(),
        frontmatter: frontmatter(built_in, name)?,
    })
}

/// The text of `relative` under `base`, or none when nothing is there. Read
/// through symlinks, only from inside `base`, and only up to the ceiling.
fn read_in(base: &Path, relative: &str) -> Result<Option<(PathBuf, String)>, String> {
    let path = base.join(relative);
    if path.symlink_metadata().is_err() {
        return Ok(None);
    }
    let resolved = devpit_core::tree::resolve(base, relative).map_err(|err| match err {
        devpit_core::TreeError::Outside { .. } => "it leads outside its folder".to_owned(),
        _ => "it could not be read".to_owned(),
    })?;
    match std::fs::metadata(&resolved) {
        Ok(meta) if !meta.is_file() => return Err("it is not a file".to_owned()),
        Ok(_) => {}
        Err(_) => return Err("it could not be read".to_owned()),
    }
    let too_large = || format!("it is larger than {} KB", MOST_BYTES / 1024);
    let file = std::fs::File::open(&resolved).map_err(|_| "it could not be read".to_owned())?;
    let raw = devpit_core::data_files::read_within(file, MOST_BYTES)
        .map_err(|_| "it could not be read".to_owned())?
        .ok_or_else(too_large)?;
    let text = String::from_utf8(raw).map_err(|_| "it is not UTF-8 text".to_owned())?;
    Ok(Some((path, text)))
}

/// The frontmatter of a SKILL.md, which must name it `name` and say when it
/// is for: the extension's entry carries it, and a host checks it.
fn frontmatter(text: &str, name: &str) -> Result<Value, String> {
    let yaml = fenced(text).ok_or("it does not begin with a --- frontmatter")?;
    let front = folder::as_json(yaml)
        .filter(Value::is_object)
        .ok_or("its frontmatter is not YAML fields")?;
    match front.get("name").and_then(Value::as_str) {
        Some(said) if said == name => {}
        Some(said) => return Err(format!("its frontmatter names it `{said}`, not `{name}`")),
        None => return Err("its frontmatter has no name".to_owned()),
    }
    if front
        .get("description")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
    {
        return Err("its frontmatter has no description".to_owned());
    }
    Ok(front)
}

/// The YAML between the opening `---` and the next line that is only `---`.
fn fenced(text: &str) -> Option<&str> {
    let body = text
        .strip_prefix("---\n")
        .or_else(|| text.strip_prefix("---\r\n"))?;
    let mut at = 0;
    for line in body.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return Some(&body[..at]);
        }
        at += line.len();
    }
    None
}

impl Skill {
    /// SHA-256 of the file's bytes, as the extension writes it.
    pub fn digest(&self) -> String {
        format!("sha256:{:x}", Sha256::digest(self.text.as_bytes()))
    }

    /// The extension's entry: what `skills/list` and `skills/get` will answer
    /// once hosts ask for them. SKILL.md is the skill's only file.
    pub fn entry(&self) -> Value {
        let uri = uri(self.name);
        json!({
            "uri": uri,
            "frontmatter": self.frontmatter,
            "resources": [{ "uri": uri, "digest": self.digest(), "size": self.text.len() }],
        })
    }

    /// The entry, with where it came from.
    pub fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "stage": self.stage,
            "layer": self.layer.word(),
            "file": self.file.as_ref().map(|file| file.display().to_string()),
            "skill": self.entry(),
        })
    }
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
