//! Changing an agent's own settings file without losing a byte of it.
//!
//! devpit gives Claude Code its hooks per terminal and never writes the
//! person's settings. An agent with no per-launch way to take hooks — the
//! Gemini CLI's global settings, say — leaves only that file, and a file the
//! person keeps by hand deserves more care than "read, change, write":
//!
//! - **Read strictly.** A file that cannot be read, is not JSON, or is not an
//!   object is refused — never taken for empty, which would write over it.
//! - **Checked before writing.** What was read is compared again just before
//!   the write, and a file changed in between is refused.
//! - **Shown first.** The change is a unified diff the person sees.
//! - **Backed up.** The file as it was is kept beside it, stamped to the
//!   second, before anything is written.
//! - **Written whole.** Into a temporary file in the same folder, then renamed
//!   over — never half-written. A symlink is followed, so a dotfiles link
//!   stays a link, and the file's permissions are kept, never widened.
//! - **Undone exactly.** What devpit adds sits under a key of its own, and
//!   taking it out gives back the rest as it was.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// A settings file as read: what it holds, and the text it was read from.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub path: PathBuf,
    pub value: Map<String, Value>,
    /// The text read, or `None` when there was no file.
    original: Option<String>,
}

/// A change, before it is made: the text it writes and the diff to show.
#[derive(Debug, Clone, PartialEq)]
pub struct Planned {
    pub after: String,
    pub diff: String,
    pub changes: bool,
}

/// Reads `path` strictly: missing is empty, anything unreadable is refused.
pub fn read(path: &Path) -> Result<Loaded, String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => return Err(format!("{} could not be read: {err}", path.display())),
    };
    let value = match text
        .as_deref()
        .map(|text| text.trim_start_matches('\u{feff}'))
    {
        None => Map::new(),
        Some(text) if text.trim().is_empty() => Map::new(),
        Some(text) => match serde_json::from_str::<Value>(text) {
            Ok(Value::Object(map)) => map,
            Ok(_) => {
                return Err(format!(
                    "{} is not a JSON object; nothing was changed",
                    path.display()
                ))
            }
            Err(err) => {
                return Err(format!(
                    "{} is not valid JSON ({err}); nothing was changed",
                    path.display()
                ))
            }
        },
    };
    Ok(Loaded {
        path: path.to_owned(),
        value,
        original: text,
    })
}

/// What `change` would make of the file, and the diff from what it is.
pub fn plan(loaded: &Loaded, change: impl FnOnce(&mut Map<String, Value>)) -> Planned {
    let mut value = loaded.value.clone();
    change(&mut value);
    let after = format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(value)).unwrap_or_default()
    );
    let before = loaded.original.clone().unwrap_or_default();
    let changes = before.trim_start_matches('\u{feff}').trim_end() != after.trim_end();
    Planned {
        diff: diff(&before, &after, &loaded.path.display().to_string()),
        after,
        changes,
    }
}

/// Writes a planned change, refusing a file changed since it was read.
/// Answers the backup, when there was a file to keep.
pub fn write(loaded: &Loaded, planned: &Planned, now: &str) -> Result<Option<PathBuf>, String> {
    let target = std::fs::canonicalize(&loaded.path).unwrap_or_else(|_| loaded.path.clone());
    let current = match std::fs::read_to_string(&target) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
        Err(err) => {
            return Err(format!(
                "{} could not be read again: {err}",
                target.display()
            ))
        }
    };
    if current != loaded.original {
        return Err(format!(
            "{} changed since it was read; nothing was written — look again",
            target.display()
        ));
    }
    let dir = target.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let backup = match &current {
        Some(text) => {
            let name = format!(
                "{}.devpit-backup-{now}",
                target
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("settings")
            );
            let backup = dir.join(name);
            std::fs::write(&backup, text)
                .map_err(|err| format!("the backup could not be written: {err}"))?;
            Some(backup)
        }
        None => None,
    };
    let temporary = dir.join(format!(
        ".{}.devpit-writing",
        target
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("settings")
    ));
    std::fs::write(&temporary, &planned.after).map_err(|err| err.to_string())?;
    if let Ok(meta) = std::fs::metadata(&target) {
        let _ = std::fs::set_permissions(&temporary, meta.permissions());
    } else {
        restrict(&temporary);
    }
    std::fs::rename(&temporary, &target).map_err(|err| {
        let _ = std::fs::remove_file(&temporary);
        err.to_string()
    })?;
    Ok(backup)
}

/// A new file is the person's alone.
#[cfg(unix)]
fn restrict(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_path: &Path) {}

/// A unified diff of two texts, three lines of context around each change.
pub fn diff(before: &str, after: &str, name: &str) -> String {
    let old: Vec<&str> = before.lines().collect();
    let new: Vec<&str> = after.lines().collect();
    // The longest common subsequence, by lines: settings files are short.
    let (n, m) = (old.len(), new.len());
    let mut table = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i][j] = if old[i] == new[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }
    let mut lines: Vec<(char, &str)> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && old[i] == new[j] {
            lines.push((' ', old[i]));
            i += 1;
            j += 1;
        } else if j < m && (i == n || table[i][j + 1] >= table[i + 1][j]) {
            lines.push(('+', new[j]));
            j += 1;
        } else {
            lines.push(('-', old[i]));
            i += 1;
        }
    }
    if lines.iter().all(|(mark, _)| *mark == ' ') {
        return String::new();
    }
    let changed: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, (mark, _))| *mark != ' ')
        .map(|(at, _)| at)
        .collect();
    let mut out = format!("--- {name}\n+++ {name}\n");
    let mut shown_to = 0usize;
    for at in changed {
        let from = at.saturating_sub(3).max(shown_to);
        let to = (at + 4).min(lines.len());
        if from > shown_to && shown_to > 0 {
            out.push_str("…\n");
        }
        for (mark, text) in &lines[from..to.max(from)] {
            out.push_str(&format!("{mark}{text}\n"));
        }
        shown_to = to.max(shown_to);
    }
    out
}

/// Sets `value` at the key `ours` inside the object at `path`, making the
/// objects on the way; what is already there under other keys is kept.
pub fn put_ours(map: &mut Map<String, Value>, path: &[&str], ours: &str, value: Value) {
    let mut at = map;
    for key in path {
        let next = at
            .entry((*key).to_owned())
            .or_insert_with(|| Value::Object(Map::new()));
        if !next.is_object() {
            *next = Value::Object(Map::new());
        }
        at = next.as_object_mut().expect("just made an object");
    }
    at.insert(ours.to_owned(), value);
}

/// Takes `ours` out of the object at `path`, and the objects on the way that
/// it leaves empty — only those, so the rest is as it was before devpit.
pub fn take_ours(map: &mut Map<String, Value>, path: &[&str], ours: &str) {
    fn walk(map: &mut Map<String, Value>, path: &[&str], ours: &str) {
        match path.split_first() {
            None => {
                map.remove(ours);
            }
            Some((key, rest)) => {
                if let Some(Value::Object(inner)) = map.get_mut(*key) {
                    walk(inner, rest, ours);
                    if inner.is_empty() {
                        map.remove(*key);
                    }
                }
            }
        }
    }
    walk(map, path, ours);
}

#[cfg(test)]
#[path = "config_edit_tests.rs"]
mod tests;
