//! Handing a path to the desktop.
//!
//! No plugin: opening a file is one command per platform, and a dependency
//! that wraps three `Command::new` calls is a dependency to keep updated for
//! nothing.

use std::path::{Path, PathBuf};

use devpit_core::Store;
use devpit_rpc::{ErrorCode, RpcError};
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Opened {
    /// What was handed over, resolved. The screen shows it when opening fails
    /// so the person can go there themselves.
    pub path: String,
}

/// Whether this process may hand that path to the desktop.
///
/// Only inside a project the person registered, or inside the devpit
/// workspace. Resolved through symlinks first, for the same reason as
/// everywhere else: comparing the strings is the check that looks right and
/// is not.
pub fn openable(roots: &[PathBuf], home: &Path, path: &Path) -> Option<PathBuf> {
    let resolved = path.canonicalize().ok()?;
    let home = home.canonicalize().unwrap_or_else(|_| home.to_path_buf());
    let allowed = roots
        .iter()
        .map(|root| root.canonicalize().unwrap_or_else(|_| root.clone()))
        .chain(std::iter::once(home));
    allowed
        .into_iter()
        .any(|root| resolved.starts_with(&root))
        .then_some(resolved)
}

fn allowed(path: &str) -> Result<PathBuf, RpcError> {
    let store = Store::open_default()?;
    let roots: Vec<PathBuf> = store
        .projects()?
        .into_iter()
        .map(|row| PathBuf::from(row.root_path))
        .collect();
    let home = Store::root().map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?;
    openable(&roots, &home, Path::new(path)).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Forbidden,
            "that path is not in a project or in the devpit workspace",
        )
    })
}

fn hand_over(argv: &[&str], path: &Path) -> Result<Opened, RpcError> {
    let ran = std::process::Command::new(argv[0])
        .args(&argv[1..])
        .arg(path)
        .spawn();
    match ran {
        Ok(_) => Ok(Opened {
            path: path.display().to_string(),
        }),
        Err(err) => Err(RpcError::new(
            ErrorCode::Unsupported,
            format!("could not open {}: {err}", path.display()),
        )),
    }
}

/// The command this desktop opens things with.
fn opener() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["open"]
    } else if cfg!(target_os = "windows") {
        &["cmd", "/C", "start", ""]
    } else {
        &["xdg-open"]
    }
}

/// `path.open` — opens a file or folder in whatever the desktop uses for it.
#[tauri::command]
#[specta::specta]
pub fn path_open(path: String) -> Result<Opened, RpcError> {
    hand_over(opener(), &allowed(&path)?)
}

/// `path.reveal` — shows a file in the file manager, selected.
///
/// Different from opening it: opening a `.rs` launches an editor, and what
/// was asked for was the folder with the file highlighted in it.
#[tauri::command]
#[specta::specta]
pub fn path_reveal(path: String) -> Result<Opened, RpcError> {
    let target = allowed(&path)?;
    if cfg!(target_os = "macos") {
        return hand_over(&["open", "-R"], &target);
    }
    if cfg!(target_os = "windows") {
        return hand_over(&["explorer", "/select,"], &target);
    }
    // No portable "select this file" on Linux; the folder is the honest answer.
    let folder = if target.is_dir() {
        target.clone()
    } else {
        target.parent().unwrap_or(&target).to_path_buf()
    };
    hand_over(opener(), &folder)
}

#[cfg(test)]
#[path = "reveal_tests.rs"]
mod tests;
