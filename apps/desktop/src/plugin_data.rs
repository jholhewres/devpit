//! A plugin's files, by name, inside its own folder of one project.
//!
//! Only while the plugin is on. A name is checked before the disk is touched;
//! the folder is resolved through symlinks inside the workspace's `projects/`,
//! a link at the file itself is refused, and a size has its ceiling before
//! anything is read.
//!
//! One gap is accepted: Tauri deserialises `text` whole before `write` can
//! compare it with the ceiling. The only caller is the webview, which already
//! drives terminals; closing it takes a command that reads the raw body.

use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};

use devpit_core::data_files;
use devpit_core::home::{plain_data_name, projects_dir};
use devpit_core::{paths, tree, Store, TreeError};
use devpit_rpc::{
    validate, CardDetail, ErrorCode, PluginFile, PluginFileRemoved, PluginFileSaved,
    PluginFileText, PluginFiles, PluginManifest, RpcError,
};

use crate::cards::card_detail;
use crate::files::modified;
use crate::plugins::manifest;
use crate::projects::{home_of, store};

/// One change at a time: comparing the mtime and renaming over the file are
/// not one step.
static CHANGING: Mutex<()> = Mutex::new(());

/// The most entries one listing reads: a folder something filled with junk
/// costs the main thread a bounded walk.
const LISTED_AT_MOST: usize = 10_000;

/// A plugin's folder in one project, once the plugin is known and on.
struct Folder {
    manifest: PluginManifest,
    /// Bytes, from a manifest `validate` passed.
    ceiling: u64,
    /// The workspace's `projects/`, which containment is measured from: a
    /// project folder that is itself a link out is outside too.
    projects: PathBuf,
    /// Relative to `projects`.
    relative: String,
}

fn invalid(message: impl Into<String>) -> RpcError {
    RpcError::new(ErrorCode::Invalid, message)
}

fn absent(err: &TreeError) -> bool {
    matches!(err, TreeError::Unreadable { source, .. } if source.kind() == ErrorKind::NotFound)
}

/// A refusal about the folder, in words that name no path: `TreeError`
/// carries absolute paths and an escaping link's target, which the page has
/// no use for. The detail goes to the log. `name` is the manifest's, the one
/// the person sees.
fn refusal(name: &str, err: TreeError) -> RpcError {
    eprintln!("plugin data refused: {err}");
    match err {
        TreeError::Unreadable { .. } if absent(&err) => {
            RpcError::new(ErrorCode::NotFound, format!("{name}'s folder is not there"))
        }
        TreeError::Unreadable { .. } => {
            RpcError::internal(format!("{name}'s folder could not be read"))
        }
        TreeError::Outside { .. } => {
            RpcError::forbidden(format!("{name}'s folder leads outside the project"))
        }
        TreeError::AlreadyExists { .. } => RpcError::forbidden(format!("{name}'s folder is taken")),
    }
}

/// One plain file name, ending in an extension the manifest declares.
pub(crate) fn check_name(manifest: &PluginManifest, name: &str) -> Result<(), RpcError> {
    if !plain_data_name(name) {
        return Err(invalid(format!("{name:?} is not a file name")));
    }
    // No leading dot, so the name is never the bare extension.
    let declared = manifest
        .data
        .extensions
        .iter()
        .any(|extension| name.ends_with(extension.as_str()));
    if !declared {
        let allowed = manifest.data.extensions.join(", ");
        return Err(invalid(format!("{name:?} does not end in {allowed}")));
    }
    Ok(())
}

/// The manifest's ceiling as bytes. Validated here as well as at startup,
/// because a release build only logs a catalogue that breaks the contract.
pub(crate) fn ceiling_of(manifest: &PluginManifest) -> Result<u64, RpcError> {
    validate(manifest).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(manifest.data.max_bytes as u64)
}

fn folder(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
) -> Result<Folder, RpcError> {
    let manifest = manifest(plugin_id)?;
    let ceiling = ceiling_of(&manifest)?;
    let home = home_of(store, root, project_id)?;
    let has = |ids: Vec<String>| ids.iter().any(|id| id == plugin_id);
    if !has(store.enabled_plugins(project_id)?) {
        let why = if has(store.installed_plugins(project_id)?) {
            "is off"
        } else {
            "is not installed"
        };
        return Err(RpcError::forbidden(format!(
            "{} {why} in this project",
            manifest.name
        )));
    }
    let data = home
        .plugin_data(plugin_id)
        .map_err(|err| invalid(err.to_string()))?;
    Ok(Folder {
        manifest,
        ceiling,
        projects: projects_dir(root),
        relative: format!("{}/{data}", home.folder()),
    })
}

/// The folder as it is on disk, or `None` before anything was written.
fn located(folder: &Folder) -> Result<Option<PathBuf>, RpcError> {
    match tree::resolve(&folder.projects, &folder.relative) {
        Ok(dir) => Ok(Some(dir)),
        Err(err) if absent(&err) => Ok(None),
        Err(err) => Err(refusal(&folder.manifest.name, err)),
    }
}

/// Makes the folder a segment at a time, each resolved inside `projects/`
/// before it is made.
fn made(folder: &Folder) -> Result<PathBuf, RpcError> {
    std::fs::create_dir_all(&folder.projects).map_err(|err| RpcError::internal(err.to_string()))?;
    let refused = |err| refusal(&folder.manifest.name, err);
    let mut walked = String::new();
    for segment in folder.relative.split('/') {
        if !walked.is_empty() {
            walked.push('/');
        }
        walked.push_str(segment);
        let at = paths::resolve_new(&folder.projects, &walked).map_err(refused)?;
        if at.symlink_metadata().is_err() {
            std::fs::create_dir(&at).map_err(|err| RpcError::internal(err.to_string()))?;
        }
    }
    tree::resolve(&folder.projects, &folder.relative).map_err(refused)
}

/// `data_files::read_within` as UTF-8 text, each refusal naming the file.
pub(crate) fn text_within(reader: impl Read, ceiling: u64, name: &str) -> Result<String, RpcError> {
    let raw = data_files::read_within(reader, ceiling)
        .map_err(|err| RpcError::internal(format!("{name} could not be read: {err}")))?
        .ok_or_else(|| invalid(format!("{name} is larger than {ceiling} bytes")))?;
    String::from_utf8(raw).map_err(|_| invalid(format!("{name} is not text")))
}

/// The file at `name` in a resolved folder, or `None` when nothing is there.
///
/// A link is refused even when it stays inside, so read, write and delete
/// agree with the listing, which never shows one; anything but a regular file
/// is refused before it is opened, since opening a FIFO waits for a writer.
/// `name` is one plain segment, so with no link at it the path stays inside.
fn file_at(dir: &Path, name: &str) -> Result<Option<PathBuf>, RpcError> {
    let path = dir.join(name);
    match std::fs::symlink_metadata(&path) {
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(RpcError::internal(format!(
            "{name} could not be read: {err}"
        ))),
        Ok(meta) if meta.file_type().is_symlink() => Err(RpcError::forbidden(format!(
            "{name} is a link, and a link is never opened here"
        ))),
        Ok(meta) if meta.is_file() => Ok(Some(path)),
        Ok(_) => Err(invalid(format!("{name} is not a file"))),
    }
}

pub(crate) fn list(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
) -> Result<PluginFiles, RpcError> {
    let folder = folder(store, root, project_id, plugin_id)?;
    let Some(dir) = located(&folder)? else {
        return Ok(PluginFiles { files: Vec::new() });
    };
    files_in(&dir, &folder.manifest, LISTED_AT_MOST)
}

/// The plugin's files in `dir`, from no more than `at_most` entries.
pub(crate) fn files_in(
    dir: &Path,
    manifest: &PluginManifest,
    at_most: usize,
) -> Result<PluginFiles, RpcError> {
    let entries = std::fs::read_dir(dir).map_err(|err| RpcError::internal(err.to_string()))?;
    let mut files = Vec::new();
    for entry in entries.take(at_most).flatten() {
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        // Regular files only: a symlink is nothing this API made.
        let Ok(meta) = std::fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if meta.is_file() && check_name(manifest, &name).is_ok() {
            files.push(PluginFile {
                bytes: meta.len() as f64,
                modified: modified(&entry.path()),
                name,
            });
        }
    }
    files.sort_by(|one, other| one.name.cmp(&other.name));
    Ok(PluginFiles { files })
}

/// A file that is there now, found the one way read and pin both find it.
fn existing(folder: &Folder, name: &str) -> Result<PathBuf, RpcError> {
    check_name(&folder.manifest, name)?;
    let missing = || RpcError::new(ErrorCode::NotFound, format!("no file {name}"));
    let dir = located(folder)?.ok_or_else(missing)?;
    file_at(&dir, name)?.ok_or_else(missing)
}

pub(crate) fn read(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
    name: &str,
) -> Result<PluginFileText, RpcError> {
    let folder = folder(store, root, project_id, plugin_id)?;
    let path = existing(&folder, name)?;
    let file = std::fs::File::open(&path)
        .map_err(|err| RpcError::internal(format!("{name} could not be read: {err}")))?;
    if !file.metadata().is_ok_and(|meta| meta.is_file()) {
        return Err(invalid(format!("{name} is not a file")));
    }
    Ok(PluginFileText {
        text: text_within(file, folder.ceiling, name)?,
        modified: modified(&path),
        name: name.to_owned(),
    })
}

/// `expected_modified` is the mtime the caller read, or `None` for a file it
/// believes is new; anything else on disk is a `Conflict`.
pub(crate) fn write(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
    name: &str,
    text: &str,
    expected_modified: Option<f64>,
) -> Result<PluginFileSaved, RpcError> {
    let folder = folder(store, root, project_id, plugin_id)?;
    check_name(&folder.manifest, name)?;
    if text.len() as u64 > folder.ceiling {
        let ceiling = folder.ceiling;
        return Err(invalid(format!("{name} is larger than {ceiling} bytes")));
    }

    let _one = CHANGING.lock().unwrap_or_else(PoisonError::into_inner);
    let dir = made(&folder)?;
    let on_disk = file_at(&dir, name)?.map(|path| modified(&path));
    if expected_modified != on_disk {
        return Err(RpcError::new(
            ErrorCode::Conflict,
            format!("{name} changed on disk since it was read"),
        ));
    }
    data_files::replace(&dir, name, text.as_bytes())
        .map_err(|err| RpcError::internal(format!("{name} was not saved: {err}")))?;
    Ok(PluginFileSaved {
        modified: modified(&dir.join(name)),
        name: name.to_owned(),
    })
}

pub(crate) fn delete(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
    name: &str,
) -> Result<PluginFileRemoved, RpcError> {
    let folder = folder(store, root, project_id, plugin_id)?;
    check_name(&folder.manifest, name)?;
    let _one = CHANGING.lock().unwrap_or_else(PoisonError::into_inner);
    let gone = PluginFileRemoved { removed: false };
    let Some(dir) = located(&folder)? else {
        return Ok(gone);
    };
    let Some(path) = file_at(&dir, name)? else {
        return Ok(gone);
    };
    std::fs::remove_file(&path).map_err(|err| RpcError::internal(err.to_string()))?;
    Ok(PluginFileRemoved { removed: true })
}

/// Pins one of the plugin's files to a card of the same project.
///
/// A name, never a path: the page picks what to pin, this decides where it
/// is. A card elsewhere is "no such card", as for one that does not exist.
pub(crate) fn pin(
    store: &Store,
    root: &Path,
    project_id: &str,
    plugin_id: &str,
    name: &str,
    card_id: &str,
) -> Result<(), RpcError> {
    let folder = folder(store, root, project_id, plugin_id)?;
    let path = existing(&folder, name)?;
    if store.project_id_of_card(card_id)?.as_deref() != Some(project_id) {
        return Err(RpcError::new(ErrorCode::NotFound, "no such card"));
    }
    // The stem fits a label: a data name is at most as long as one.
    let label = Path::new(name).file_stem().map_or_else(
        || name.to_owned(),
        |stem| stem.to_string_lossy().into_owned(),
    );
    let at = path.display().to_string();
    store.attach(card_id, &at, &label, Some(plugin_id))?;
    Ok(())
}

/// `plugin.data.list` — the plugin's files in this project.
#[tauri::command]
#[specta::specta]
pub fn plugin_data_list(project_id: String, plugin_id: String) -> Result<PluginFiles, RpcError> {
    list(&store()?, &Store::root()?, &project_id, &plugin_id)
}

/// `plugin.data.read` — one file's text.
#[tauri::command]
#[specta::specta]
pub fn plugin_data_read(
    project_id: String,
    plugin_id: String,
    name: String,
) -> Result<PluginFileText, RpcError> {
    read(&store()?, &Store::root()?, &project_id, &plugin_id, &name)
}

/// `plugin.data.write` — saves, refusing a change it never saw.
#[tauri::command]
#[specta::specta]
pub fn plugin_data_write(
    project_id: String,
    plugin_id: String,
    name: String,
    text: String,
    expected_modified: Option<f64>,
) -> Result<PluginFileSaved, RpcError> {
    let (store, root) = (store()?, Store::root()?);
    write(
        &store,
        &root,
        &project_id,
        &plugin_id,
        &name,
        &text,
        expected_modified,
    )
}

/// `plugin.data.delete` — removes one file.
#[tauri::command]
#[specta::specta]
pub fn plugin_data_delete(
    project_id: String,
    plugin_id: String,
    name: String,
) -> Result<PluginFileRemoved, RpcError> {
    delete(&store()?, &Store::root()?, &project_id, &plugin_id, &name)
}

/// `plugin.data.pin` — pins one file to a card, answering with the card.
#[tauri::command]
#[specta::specta]
pub fn plugin_data_pin(
    project_id: String,
    plugin_id: String,
    name: String,
    card_id: String,
) -> Result<CardDetail, RpcError> {
    let (store, root) = (store()?, Store::root()?);
    pin(&store, &root, &project_id, &plugin_id, &name, &card_id)?;
    card_detail(project_id, card_id)
}

#[cfg(test)]
#[path = "plugin_data_tests.rs"]
mod tests;
