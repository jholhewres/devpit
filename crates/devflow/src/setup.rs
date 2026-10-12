//! What a setup leaves in `.devpit/devflow/`: devflow.yaml, one script per
//! check, a `.gitignore` for the runs, and the reviewer personas and findings
//! schema the review stage reads there. Previewed first, written only when
//! asked, and never committed.

use std::path::Path;

use serde::Serialize;

use crate::config::Config;
use crate::folder::{DIR, FILE};

/// The personas and the schema, as this build ships them.
const PERSONAS: [(&str, &str); 3] = [
    (
        "devflow-spec",
        include_str!("../../../flow/personas/devflow-spec.md"),
    ),
    (
        "devflow-standards",
        include_str!("../../../flow/personas/devflow-standards.md"),
    ),
    (
        "devflow-security",
        include_str!("../../../flow/personas/devflow-security.md"),
    ),
];
const FINDINGS_SCHEMA: &str = include_str!("../../../flow/schemas/devflow-findings.schema.json");

/// The line the `.gitignore` needs: evidence stays out of the repository.
const IGNORED: &str = "runs/";

/// A check's command, kept in a script of its own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Script {
    pub name: String,
    pub command: String,
    /// Where the command was found, when detection found it.
    pub source: Option<String>,
}

/// How a check runs its script, from the project's root. Through `sh`, so a
/// checkout that lost the executable bit still runs it.
pub fn script_run(name: &str) -> String {
    format!("sh {DIR}/scripts/check-{name}.sh")
}

/// `config` with every check that runs a command of its own running a script
/// instead, and the scripts to write. `sources` says where a command was found.
pub fn scripted(mut config: Config, sources: &[(String, String)]) -> (Config, Vec<Script>) {
    let mut scripts = Vec::new();
    for check in &mut config.stages.execution.checks {
        let run = script_run(&check.name);
        if check.run == run {
            continue;
        }
        scripts.push(Script {
            name: check.name.clone(),
            source: sources
                .iter()
                .find(|(command, _)| *command == check.run)
                .map(|(_, source)| source.clone()),
            command: std::mem::replace(&mut check.run, run),
        });
    }
    (config, scripts)
}

fn script_text(script: &Script) -> String {
    let from = script
        .source
        .as_deref()
        .map(|source| format!(", found in {source}"))
        .unwrap_or_default();
    format!(
        "#!/bin/sh\n\
         # Devflow check `{name}`{from}.\n\
         # It runs from the project's root, wherever it is started; a report\n\
         # written to $DEVPIT_REPORT_DIR is read as the check's evidence.\n\
         set -e\n\
         cd \"$(dirname \"$0\")/../../..\"\n\
         DEVPIT_REPORT_DIR=\"${{DEVPIT_REPORT_DIR:-{DIR}/runs/local}}\"\n\
         export DEVPIT_REPORT_DIR\n\
         mkdir -p \"$DEVPIT_REPORT_DIR\"\n\
         {command}\n",
        name = script.name,
        command = script.command,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// devpit's to write over.
    Ours,
    /// Written when absent; a copy the person changed is theirs.
    Theirs,
    /// The `.gitignore`: the line added to what is there.
    Ignore,
}

/// One file a setup writes, relative to the project's root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned {
    pub path: String,
    text: String,
    kind: Kind,
    executable: bool,
}

/// What writing a planned file does, or did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileChange {
    pub path: String,
    /// `create`, `replace`, `same`, `kept` (a copy of the person's) or `append`.
    pub change: &'static str,
}

/// Every file a setup of `yaml` with `scripts` writes.
pub fn planned(yaml: &str, scripts: &[Script]) -> Vec<Planned> {
    let at = |path: String, text: String, kind: Kind, executable: bool| Planned {
        path,
        text,
        kind,
        executable,
    };
    let mut files = vec![
        at(FILE.to_owned(), yaml.to_owned(), Kind::Ours, false),
        at(
            format!("{DIR}/.gitignore"),
            format!("# Evidence devpit keeps per run, attached to the card.\n{IGNORED}\n"),
            Kind::Ignore,
            false,
        ),
    ];
    files.extend(scripts.iter().map(|script| {
        at(
            format!("{DIR}/scripts/check-{}.sh", script.name),
            script_text(script),
            Kind::Ours,
            true,
        )
    }));
    files.extend(PERSONAS.iter().map(|(name, text)| {
        at(
            format!("{DIR}/reviewers/{name}.md"),
            (*text).to_owned(),
            Kind::Theirs,
            false,
        )
    }));
    files.push(at(
        format!("{DIR}/devflow-findings.schema.json"),
        FINDINGS_SCHEMA.to_owned(),
        Kind::Theirs,
        false,
    ));
    files
}

/// Only devflow.yaml, for a change to one field.
pub fn config_only(yaml: &str) -> Vec<Planned> {
    planned(yaml, &[]).into_iter().take(1).collect()
}

/// What writing `files` under `root` would do; nothing is written.
pub fn preview(root: &Path, files: &[Planned]) -> Vec<FileChange> {
    files
        .iter()
        .map(|file| FileChange {
            path: file.path.clone(),
            change: change_of(file, there(root, &file.path).as_deref()),
        })
        .collect()
}

/// Writes `files` under `root`, every folder and file resolved through
/// symlinks and kept inside the project. Commits nothing.
pub fn write(root: &Path, files: &[Planned]) -> Result<Vec<FileChange>, String> {
    let mut done = Vec::new();
    for file in files {
        let existing = there(root, &file.path);
        let change = change_of(file, existing.as_deref());
        let text = match (change, file.kind) {
            ("same" | "kept", _) => {
                done.push(FileChange {
                    path: file.path.clone(),
                    change,
                });
                continue;
            }
            ("append", Kind::Ignore) => {
                let mut kept = existing.unwrap_or_default();
                if !kept.ends_with('\n') {
                    kept.push('\n');
                }
                kept.push_str(IGNORED);
                kept.push('\n');
                kept
            }
            _ => file.text.clone(),
        };
        let parent = file.path.rsplit_once('/').map_or("", |(parent, _)| parent);
        folders(root, parent)?;
        let target = devpit_core::paths::resolve_new(root, &file.path)
            .map_err(|err| format!("{}: {err}", file.path))?;
        std::fs::write(&target, text).map_err(|err| format!("{}: {err}", file.path))?;
        if file.executable {
            executable(&target);
        }
        done.push(FileChange {
            path: file.path.clone(),
            change,
        });
    }
    Ok(done)
}

/// The file's text, when it is there and inside the project.
fn there(root: &Path, path: &str) -> Option<String> {
    let resolved = devpit_core::tree::resolve(root, path).ok()?;
    let meta = std::fs::metadata(&resolved).ok()?;
    (meta.is_file() && meta.len() <= crate::folder::MOST_BYTES)
        .then(|| std::fs::read_to_string(resolved).ok())
        .flatten()
}

fn change_of(file: &Planned, existing: Option<&str>) -> &'static str {
    match (existing, file.kind) {
        (None, _) => "create",
        (Some(text), Kind::Ignore) => {
            if text.lines().any(|line| line.trim() == IGNORED) {
                "same"
            } else {
                "append"
            }
        }
        (Some(text), _) if text == file.text => "same",
        (Some(_), Kind::Theirs) => "kept",
        (Some(_), _) => "replace",
    }
}

/// Makes each folder of `relative` that is missing, refusing one that
/// leads outside the project.
fn folders(root: &Path, relative: &str) -> Result<(), String> {
    let mut walked = String::new();
    for part in relative.split('/').filter(|part| !part.is_empty()) {
        if !walked.is_empty() {
            walked.push('/');
        }
        walked.push_str(part);
        let folder = devpit_core::paths::resolve_new(root, &walked)
            .map_err(|err| format!("{walked}: {err}"))?;
        if !folder.is_dir() {
            std::fs::create_dir(&folder).map_err(|err| format!("{walked}: {err}"))?;
        }
    }
    Ok(())
}

#[cfg(unix)]
fn executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
}

#[cfg(not(unix))]
fn executable(_: &Path) {}

#[cfg(test)]
#[path = "setup_tests.rs"]
mod tests;
