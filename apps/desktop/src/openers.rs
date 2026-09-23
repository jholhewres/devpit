//! The apps a folder can be handed to.
//!
//! `path.open` asks the desktop, which is right for a `.md` and wrong for a
//! project: the desktop's idea of what opens a folder is a file manager, and
//! what a person wants is their editor. So this is a short list they keep, and
//! each entry is one program.
//!
//! # One program, never a command line
//!
//! The command is stored as a program and run as a program:
//!
//! ```ignore
//! Command::new(&app.command).arg(path)
//! ```
//!
//! Not through a shell, and the path never touched to the string. The rule is
//! `AGENTS.md`'s and the reason is this file exactly: what is typed here is
//! written to disk today and executed tomorrow, so a value like
//! `code; rm -rf ~` must not be able to become two commands. It cannot,
//! because there is no shell to split it — but it is refused at the field
//! anyway, so the failure is a sentence while someone is typing rather than a
//! program that is silently never found.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use devpit_core::{preference, Store};
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

/// One app in the list.
#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenApp {
    /// Stable, and what the menu is keyed on.
    pub id: String,
    pub label: String,
    /// The program, as it would be typed into a terminal. One word.
    pub command: String,
    /// Whether this machine can actually run it. Asked of the shell, not of
    /// `PATH`: an editor launched by a shell function has no file on `PATH`
    /// and would be greyed out for no reason.
    pub installed: bool,
}

/// An app this build knows how to offer before anyone has typed anything.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KnownApp {
    pub id: String,
    pub label: String,
    pub command: String,
}

/// The ones the Add menu offers, plus whatever is typed as a custom app.
const KNOWN: &[(&str, &str, &str)] = &[
    ("vscode", "VS Code", "code"),
    ("cursor", "Cursor", "cursor"),
    ("zed", "Zed", "zed"),
    ("windsurf", "Windsurf", "windsurf"),
    ("sublime", "Sublime Text", "subl"),
    ("intellij", "IntelliJ IDEA", "idea"),
];

/// What a stored entry looks like on disk — everything but `installed`, which
/// is measured on every read and would be a lie the moment it were saved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    id: String,
    label: String,
    command: String,
}

fn store() -> Result<Store, RpcError> {
    Ok(Store::open_default()?)
}

/// Whether a string is a program and not a command line.
///
/// A program name, or a path to one. Anything a shell would treat as
/// punctuation is refused — and so is whitespace, because `code --wait` is two
/// things and this field holds one.
pub fn is_a_program(command: &str) -> bool {
    let command = command.trim();
    !command.is_empty()
        && command.len() <= 512
        && !command.chars().any(|letter| {
            letter.is_whitespace()
                || letter.is_control()
                || ";|&$`<>()[]{}*?!#'\"\\\n".contains(letter)
        })
}

/// A usable id built from a label, for a custom app.
fn id_from(label: &str) -> String {
    // Runs collapse: `C++ IDE` is four unusable characters in a row, and
    // `c---ide` is an id nobody would recognise as the thing they named.
    let mut kept = String::new();
    for letter in label.trim().to_lowercase().chars() {
        if letter.is_ascii_alphanumeric() {
            kept.push(letter);
        } else if !kept.ends_with('-') {
            kept.push('-');
        }
    }
    let trimmed = kept.trim_matches('-').to_owned();
    if trimmed.is_empty() {
        format!("app-{}", ulid::Ulid::generate().to_string().to_lowercase())
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn stored(store: &Store) -> Result<Vec<Stored>, RpcError> {
    let raw = store
        .preference(preference::OPEN_IN_APPS)?
        .unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    // A preference that cannot be parsed is an empty list, not an error: it
    // is one row in a settings table and refusing to draw the pane over it
    // would make a typo unrecoverable from inside the app.
    Ok(serde_json::from_str(&raw).unwrap_or_default())
}

fn save(store: &Store, apps: &[Stored]) -> Result<(), RpcError> {
    let written = serde_json::to_string(apps).map_err(|err| RpcError::internal(err.to_string()))?;
    store.set_preference(preference::OPEN_IN_APPS, &written)?;
    Ok(())
}

/// Which of these commands this machine can run.
///
/// One shell, one question, all of them at once: a probe per app would be six
/// shell startups, and `$SHELL -ic` costs well over a second on a dotfile-rich
/// machine.
fn present(commands: &[String]) -> HashSet<String> {
    if commands.is_empty() {
        return HashSet::new();
    }
    let asked = commands
        .iter()
        .filter(|command| is_a_program(command))
        .map(|command| format!("command -v {command} >/dev/null 2>&1 && echo {command}"))
        .collect::<Vec<_>>()
        .join("; ");

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    let Ok(output) = devpit_pty::host_env::command(shell)
        .args(["-ic", &asked])
        // Nulled, or an interactive shell that decides to read from it hangs
        // this process for as long as the window is open.
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    else {
        return HashSet::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| line.trim().to_owned())
        .filter(|line| !line.is_empty())
        .collect()
}

fn drawn(apps: Vec<Stored>) -> Vec<OpenApp> {
    let here = present(
        &apps
            .iter()
            .map(|app| app.command.clone())
            .collect::<Vec<_>>(),
    );
    apps.into_iter()
        .map(|app| OpenApp {
            installed: here.contains(&app.command),
            id: app.id,
            label: app.label,
            command: app.command,
        })
        .collect()
}

/// `apps.list` — the apps in the Open in menu.
#[tauri::command]
#[specta::specta]
pub async fn apps_list() -> Result<Vec<OpenApp>, RpcError> {
    // Off the UI thread: `present` runs a shell, and the settings pane must
    // not hold the window while a `.zshrc` reads the network.
    let apps = tauri::async_runtime::spawn_blocking(|| stored(&store()?).map(drawn))
        .await
        .map_err(|err| RpcError::internal(err.to_string()))??;
    Ok(apps)
}

/// `apps.known` — what the Add menu offers.
#[tauri::command]
#[specta::specta]
pub async fn apps_known() -> Result<Vec<KnownApp>, RpcError> {
    crate::off_main::blocking(apps_known_now).await
}

/// [`apps_known`], on the calling thread.
pub(crate) fn apps_known_now() -> Result<Vec<KnownApp>, RpcError> {
    Ok(KNOWN
        .iter()
        .map(|(id, label, command)| KnownApp {
            id: (*id).to_owned(),
            label: (*label).to_owned(),
            command: (*command).to_owned(),
        })
        .collect())
}

/// `apps.add` — puts one in the list.
///
/// Adding the same app twice is not an error and not a duplicate: it updates
/// the one that is there, which is what someone repeating themselves means.
#[tauri::command]
#[specta::specta]
pub async fn apps_add(label: String, command: String) -> Result<Vec<OpenApp>, RpcError> {
    let label = label.trim().to_owned();
    if label.is_empty() {
        return Err(RpcError::new(ErrorCode::Invalid, "an app needs a name"));
    }
    if label.chars().count() > 64 {
        return Err(RpcError::new(ErrorCode::Invalid, "that name is too long"));
    }
    let command = command.trim().to_owned();
    if !is_a_program(&command) {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "that has to be one program, with no arguments and no shell punctuation",
        ));
    }

    tauri::async_runtime::spawn_blocking(move || {
        let store = store()?;
        let mut apps = stored(&store)?;
        let id = KNOWN
            .iter()
            .find(|(_, _, known)| *known == command)
            .map(|(id, _, _)| (*id).to_owned())
            .unwrap_or_else(|| id_from(&label));

        match apps.iter_mut().find(|app| app.id == id) {
            Some(had) => {
                had.label = label;
                had.command = command;
            }
            None => apps.push(Stored { id, label, command }),
        }
        save(&store, &apps)?;
        Ok::<_, RpcError>(drawn(apps))
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))?
}

/// `apps.remove` — takes one out. Nothing on disk is touched.
#[tauri::command]
#[specta::specta]
pub async fn apps_remove(app_id: String) -> Result<Vec<OpenApp>, RpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        let store = store()?;
        let mut apps = stored(&store)?;
        apps.retain(|app| app.id != app_id);
        save(&store, &apps)?;
        Ok::<_, RpcError>(drawn(apps))
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))?
}

/// `apps.open` — hands a folder to one of them.
///
/// The path is checked the same way `path.open` checks it: inside a registered
/// project or inside the devpit workspace, resolved through symlinks. This
/// process runs terminals, and reaching it is reaching the machine.
#[tauri::command]
#[specta::specta]
pub async fn apps_open(app_id: String, path: String) -> Result<(), RpcError> {
    crate::off_main::blocking(move || apps_open_now(app_id, path)).await
}

/// [`apps_open`], on the calling thread.
pub(crate) fn apps_open_now(app_id: String, path: String) -> Result<(), RpcError> {
    let store = store()?;
    let app = stored(&store)?
        .into_iter()
        .find(|app| app.id == app_id)
        .ok_or_else(|| RpcError::new(ErrorCode::NotFound, "no such app"))?;

    // Read back and checked again rather than trusted because it was checked
    // on the way in: the preference file is on disk and a write this process
    // did not make is a write this process must not execute.
    if !is_a_program(&app.command) {
        return Err(RpcError::new(
            ErrorCode::Forbidden,
            "that app's command is not a program",
        ));
    }

    let folder = allowed(&store, &path)?;
    devpit_pty::host_env::command(&app.command)
        .arg(&folder)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| {
            RpcError::new(
                ErrorCode::Unsupported,
                format!("could not run {}: {err}", app.command),
            )
        })?;
    Ok(())
}

/// The same gate `path.open` uses, asked here rather than imported, because
/// this one takes a `Store` that is already open.
fn allowed(store: &Store, path: &str) -> Result<PathBuf, RpcError> {
    let roots: Vec<PathBuf> = store
        .projects()?
        .into_iter()
        .map(|row| PathBuf::from(row.root_path))
        .collect();
    let home = Store::root()?;
    crate::reveal::openable(&roots, &home, Path::new(path)).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Forbidden,
            "that path is not in a project or in the devpit workspace",
        )
    })
}

#[cfg(test)]
#[path = "openers_tests.rs"]
mod tests;
