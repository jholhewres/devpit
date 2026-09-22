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
/// Only inside a project the person registered, inside the devpit workspace,
/// or inside the CLI's own configuration — the three places this window draws
/// file paths from. Resolved through symlinks first, for the same reason as
/// everywhere else: comparing the strings is the check that looks right and
/// is not.
pub(crate) fn openable(roots: &[PathBuf], home: &Path, path: &Path) -> Option<PathBuf> {
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
    let mut roots: Vec<PathBuf> = store
        .projects()?
        .into_iter()
        .map(|row| PathBuf::from(row.root_path))
        .collect();
    // Every installation of the CLI, because the Skills panel lists files out
    // of them and offers to open them. Every `SKILL.md` lives under one of
    // these and nothing else, so without this the buttons on that panel
    // refused every path they were ever handed — measured, not reasoned about.
    roots.extend(
        crate::installations::found()
            .unwrap_or_default()
            .into_iter()
            .map(|one| one.directory),
    );
    let home = Store::root().map_err(|err| RpcError::new(ErrorCode::Internal, err.to_string()))?;
    allowed_in(&store, &roots, &home, path)
}

/// `allowed`, once this machine's roots are known.
///
/// A transcript keeps the path a picture was pasted at, which may predate its
/// project's folder getting a name, so the named folder is looked in first.
pub(crate) fn allowed_in(
    store: &Store,
    roots: &[PathBuf],
    home: &Path,
    path: &str,
) -> Result<PathBuf, RpcError> {
    let path = devpit_core::home::moved(store, home, Path::new(path))
        .unwrap_or_else(|| PathBuf::from(path));
    openable(roots, home, &path).ok_or_else(|| {
        RpcError::new(
            ErrorCode::Forbidden,
            "that path is not in a project, the devpit workspace, or the CLI's configuration",
        )
    })
}

fn hand_over(
    argv: &[&str],
    what: impl AsRef<std::ffi::OsStr>,
    shown: &str,
) -> Result<Opened, RpcError> {
    let ran = devpit_pty::host_env::command(argv[0])
        .args(&argv[1..])
        .arg(what)
        .spawn();
    match ran {
        Ok(_) => Ok(Opened {
            path: shown.to_owned(),
        }),
        Err(err) => Err(RpcError::new(
            ErrorCode::Unsupported,
            format!("could not open {shown}: {err}"),
        )),
    }
}

/// The web address this process will hand to the desktop, or why it will not.
///
/// `https` and nothing else: `file:` would open anything on the disk, and a
/// string starting with `-` is read by the opener as a flag rather than as an
/// address. There is no list of allowed hosts on purpose — an agent's homepage
/// comes from the person's own files in `~/.devpit/agents/`, so a fixed list
/// would refuse the addresses this exists to open. What is refused is a scheme
/// that does something other than open a page, and a host somebody could hide
/// credentials in.
pub(crate) fn web_address(url: &str) -> Result<&str, RpcError> {
    let refuse = |why: &str| {
        Err(RpcError::new(
            ErrorCode::Forbidden,
            format!("that address is not one devpit opens: {why}"),
        ))
    };
    let Some(rest) = url.strip_prefix("https://") else {
        return refuse("only https");
    };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if host.is_empty() {
        return refuse("no host");
    }
    if host.contains('@') {
        return refuse("a host cannot carry credentials");
    }
    if url.contains(['\n', '\r', ' ']) {
        return refuse("an address has no spaces or line breaks in it");
    }
    Ok(url)
}

/// `url.open` — opens a web address in the browser the person uses.
///
/// The window cannot do this itself: a `target="_blank"` reaches wry's
/// `new_window_req_handler`, which tauri never sets (wry 0.55.1
/// `webkitgtk/mod.rs:487`), so the click did nothing at all.
#[tauri::command]
#[specta::specta]
pub fn url_open(url: String) -> Result<Opened, RpcError> {
    let address = web_address(&url)?;
    hand_over(opener(), address, address)
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
    let target = allowed(&path)?;
    let shown = target.display().to_string();
    hand_over(opener(), &target, &shown)
}

/// `path.reveal` — shows a file in the file manager, selected.
///
/// Different from opening it: opening a `.rs` launches an editor, and what
/// was asked for was the folder with the file highlighted in it.
#[tauri::command]
#[specta::specta]
pub fn path_reveal(path: String) -> Result<Opened, RpcError> {
    let target = allowed(&path)?;
    let shown = target.display().to_string();
    if cfg!(target_os = "macos") {
        return hand_over(&["open", "-R"], &target, &shown);
    }
    if cfg!(target_os = "windows") {
        return hand_over(&["explorer", "/select,"], &target, &shown);
    }
    // No portable "select this file" on Linux; the folder is the honest answer.
    let folder = if target.is_dir() {
        target.clone()
    } else {
        target.parent().unwrap_or(&target).to_path_buf()
    };
    hand_over(opener(), &folder, &shown)
}

#[cfg(test)]
#[path = "reveal_tests.rs"]
mod tests;
