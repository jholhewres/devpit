//! Telling Claude Code a folder is trusted before a session opens there.
//!
//! The CLI asks once per folder and per account whether the folder is trusted,
//! and a session stopped on that question is not up: it registers nowhere and
//! only someone at the screen can let it go on. Adding a project to devpit is
//! that consent, so devpit writes down the answer the CLI would.
//!
//! Measured on Claude Code 2.1.294: the answer is looked up from the folder to
//! the root of its git repository and no further, and a worktree is a root of
//! its own. So every project and every worktree is written by its own path.
//!
//! The file is the CLI's, and the CLI rewrites it all the time: only the one
//! key changes, and a file that changed while this was writing is read again.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

const KEY: &str = "hasTrustDialogAccepted";
/// Reads of a file that kept changing before this gives up.
const ATTEMPTS: usize = 3;

/// The key the CLI files a folder under: its path, with `/` on Windows too.
pub fn key_of(folder: &Path) -> String {
    let said = folder.to_string_lossy();
    if cfg!(windows) {
        said.trim_start_matches(r"\\?\").replace('\\', "/")
    } else {
        said.into_owned()
    }
}

/// Whether `config` trusts `key`, by that exact path.
pub fn trusts(config: &Map<String, Value>, key: &str) -> bool {
    config
        .get("projects")
        .and_then(|projects| projects.get(key))
        .and_then(|one| one.get(KEY))
        .and_then(Value::as_bool)
        == Some(true)
}

/// Marks every key trusted in `config`; answers whether anything changed.
/// A `projects` that is not an object is refused rather than replaced.
pub fn trusted(config: &mut Map<String, Value>, keys: &[String]) -> Result<bool, String> {
    let mut changed = false;
    for key in keys {
        if trusts(config, key) {
            continue;
        }
        let projects = config
            .entry("projects")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or("its `projects` is not an object")?;
        let one = projects
            .entry(key.clone())
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| format!("its entry for {key} is not an object"))?;
        one.insert(KEY.to_owned(), Value::Bool(true));
        changed = true;
    }
    Ok(changed)
}

/// Trusts `keys` in the CLI's settings file `file`; answers whether it wrote.
///
/// A file that is not there is left alone: that account has never run, and
/// its first start writes one of its own.
pub fn trust_in(file: &Path, keys: &[String]) -> Result<bool, String> {
    trust_with(file, keys, || {})
}

/// [`trust_in`], with `meanwhile` run between writing and checking: where the
/// CLI's own writes land, which is what a test needs to stand in for.
fn trust_with(file: &Path, keys: &[String], mut meanwhile: impl FnMut()) -> Result<bool, String> {
    let target = match std::fs::canonicalize(file) {
        Ok(target) => target,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(err) => return Err(format!("{}: {err}", file.display())),
    };
    for _ in 0..ATTEMPTS {
        let before = read(&target)?;
        let mut config = parsed(&target, &before)?;
        if !trusted(&mut config, keys)? {
            return Ok(false);
        }
        let after =
            serde_json::to_string_pretty(&Value::Object(config)).map_err(|err| err.to_string())?;
        let temporary = beside(&target);
        std::fs::write(&temporary, after).map_err(|err| err.to_string())?;
        if let Ok(meta) = std::fs::metadata(&target) {
            let _ = std::fs::set_permissions(&temporary, meta.permissions());
        }
        meanwhile();
        // Written by the CLI meanwhile: what was read is stale, so start over.
        if read(&target)? != before {
            let _ = std::fs::remove_file(&temporary);
            continue;
        }
        return std::fs::rename(&temporary, &target)
            .map(|_| true)
            .map_err(|err| {
                let _ = std::fs::remove_file(&temporary);
                err.to_string()
            });
    }
    Err(format!(
        "{} kept changing while devpit wrote to it",
        target.display()
    ))
}

fn read(target: &Path) -> Result<String, String> {
    std::fs::read_to_string(target).map_err(|err| format!("{}: {err}", target.display()))
}

/// The file as an object, or why not: never taken for empty.
fn parsed(target: &Path, text: &str) -> Result<Map<String, Value>, String> {
    match serde_json::from_str::<Value>(text.trim_start_matches('\u{feff}')) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(format!("{} is not a JSON object", target.display())),
        Err(err) => Err(format!("{} is not valid JSON ({err})", target.display())),
    }
}

/// A temporary file in the same folder, so the rename never crosses disks.
fn beside(target: &Path) -> PathBuf {
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    target.with_file_name(format!(".{name}.devpit-{}", std::process::id()))
}

#[cfg(test)]
#[path = "trust_tests.rs"]
mod tests;
