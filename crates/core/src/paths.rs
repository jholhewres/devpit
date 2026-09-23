//! Resolving a path that may not exist — because it was just deleted, or has
//! not been created yet.
//!
//! `tree::resolve` canonicalises the whole path, which requires it to already
//! be on disk. A deleted file a discard wants to restore, or a file a create
//! is about to write, is exactly the case where that requirement is wrong: the
//! path is still one this project owns and still has to be checked, but there
//! is nothing there yet to canonicalise. This resolves the parent — which does
//! exist — through symlinks and containment the same way `resolve` does, then
//! checks the final segment by name: no separator, no `..`, not empty, and no
//! `.git` in any segment, before or after symlinks. A name that exists goes
//! through `resolve` itself, so a symlink there is caught; a symlink planted
//! *after* this check is a race this does not close.

use std::path::{Path, PathBuf};

use crate::tree::{resolve, TreeError};

/// Whether any segment of `relative` is `.git`, in any case (`.GIT` is the
/// same folder on macOS). `pub(crate)` so `paths_tests.rs` can call it.
pub(crate) fn touches_git(relative: &str) -> bool {
    relative
        .split('/')
        .any(|segment| segment.eq_ignore_ascii_case(".git"))
}

/// Whether `resolved`, already canonical, sits inside `.git` below `root` —
/// the string check misses a symlinked folder that leads there.
fn under_git(root: &Path, resolved: &Path) -> bool {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    resolved.strip_prefix(&root).is_ok_and(|rest| {
        rest.components()
            .any(|part| part.as_os_str().eq_ignore_ascii_case(".git"))
    })
}

fn clear_of_git(root: &Path, resolved: PathBuf) -> Result<PathBuf, TreeError> {
    if under_git(root, &resolved) {
        return Err(TreeError::Outside { path: resolved });
    }
    Ok(resolved)
}

/// The parent resolved through symlinks, with the final name joined as
/// written. The root however it is spelled — `""`, `.`, `sub/..` — ends in an
/// empty, `.` or `..` name, and is refused here.
fn beside_parent(root: &Path, relative: &str) -> Result<PathBuf, TreeError> {
    let relative = relative.trim_end_matches('/');
    let (parent, name) = relative.rsplit_once('/').unwrap_or(("", relative));
    if name.is_empty() || name == "." || name == ".." || touches_git(relative) {
        return Err(TreeError::Outside {
            path: root.join(relative),
        });
    }
    Ok(resolve(root, parent)?.join(name))
}

/// Resolves `relative` against `root` without requiring it to exist.
pub fn resolve_new(root: &Path, relative: &str) -> Result<PathBuf, TreeError> {
    let candidate = beside_parent(root, relative)?;

    // `symlink_metadata`, not `exists`: `exists` follows the link and asks
    // about its *target*, so a broken symlink — pointing at nothing — reads
    // as absent and would fall through unexamined. A name is "already there"
    // the moment something sits at it, valid or not, and only `resolve`'s own
    // canonicalise-then-check on the whole path can vouch for what it is. A
    // broken link fails that canonicalisation and is refused, correctly: the
    // name is taken by something this cannot vouch for.
    if candidate.symlink_metadata().is_ok() {
        return clear_of_git(root, resolve(root, relative)?);
    }
    clear_of_git(root, candidate)
}

/// An existing file whose contents may be rewritten: `resolve`, and nowhere
/// inside `.git`, where a written hook is code git runs.
pub fn resolve_writable(root: &Path, relative: &str) -> Result<PathBuf, TreeError> {
    clear_of_git(root, resolve(root, relative)?)
}

/// The entry at `relative` itself, a symlink kept as the link: what a delete
/// or a move acts on is the row that was clicked, never what it points at.
fn resolve_entry(root: &Path, relative: &str) -> Result<PathBuf, TreeError> {
    let entry = beside_parent(root, relative)?;
    entry
        .symlink_metadata()
        .map_err(|source| TreeError::Unreadable {
            path: entry.clone(),
            source,
        })?;
    clear_of_git(root, entry)
}

/// Creates an empty file, or a folder, at `relative`.
///
/// Refuses when the name is already taken: `resolve_new` hands back the
/// canonical existing path in that case, and creating is not the same
/// operation as overwriting.
pub fn create(root: &Path, relative: &str, is_dir: bool) -> Result<(), TreeError> {
    let target = resolve_new(root, relative)?;
    if target.symlink_metadata().is_ok() {
        return Err(TreeError::AlreadyExists { path: target });
    }

    let made = if is_dir {
        std::fs::create_dir(&target)
    } else {
        std::fs::write(&target, [])
    };
    made.map_err(|source| TreeError::Unreadable {
        path: target,
        source,
    })
}

/// Moves or renames `from` to `to` — the same operation either way, since a
/// rename is a move within one directory.
///
/// `from` must already be there, and a symlink moves as the link. `to` is
/// refused when it is already taken, the same check `create` makes.
pub fn move_to(root: &Path, from: &str, to: &str) -> Result<(), TreeError> {
    let source = resolve_entry(root, from)?;
    let destination = resolve_new(root, to)?;
    if destination.symlink_metadata().is_ok() {
        return Err(TreeError::AlreadyExists { path: destination });
    }

    std::fs::rename(&source, &destination).map_err(|err| TreeError::Unreadable {
        path: destination,
        source: err,
    })
}

/// Removes whatever is at `relative` — a file, or a folder and everything
/// under it.
///
/// A symlink is removed as the link: `is_dir` would follow it, and
/// `remove_dir_all` would then empty the folder it points at.
pub fn remove(root: &Path, relative: &str) -> Result<(), TreeError> {
    let target = resolve_entry(root, relative)?;
    let removed = if target.symlink_metadata().is_ok_and(|meta| meta.is_dir()) {
        std::fs::remove_dir_all(&target)
    } else {
        std::fs::remove_file(&target)
    };
    removed.map_err(|source| TreeError::Unreadable {
        path: target,
        source,
    })
}
