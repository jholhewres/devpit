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
//! `.git` in any segment — the repository's own database is not a file this
//! hands out a writable path to. A name that already exists is handed to
//! `resolve` itself instead of assembled by hand, so a symlink sitting there
//! is caught by the same canonicalise-then-check every other path goes
//! through; only a name genuinely absent falls back to the parent-only
//! answer, which is the one case `resolve` cannot make. A symlink planted at
//! that exact name *after* this check is a race this does not close, the same
//! one every check-then-write here already lives with.

use std::path::{Path, PathBuf};

use crate::tree::{resolve, TreeError};

/// Whether any segment of `relative` is `.git` — the repository's own
/// database, not a file this hands out a writable path to.
///
/// `pub(crate)`, not private: `paths_tests.rs` is a sibling file, the same
/// shape as `tree_tests.rs`, and calls this directly rather than only
/// reaching it through `resolve_new`.
pub(crate) fn touches_git(relative: &str) -> bool {
    relative.split('/').any(|segment| segment == ".git")
}

/// Resolves `relative` against `root` without requiring it to exist.
pub fn resolve_new(root: &Path, relative: &str) -> Result<PathBuf, TreeError> {
    let relative = relative.trim_end_matches('/');
    let (parent, name) = match relative.rsplit_once('/') {
        Some((parent, name)) => (parent, name),
        None => ("", relative),
    };

    if name.is_empty() || name == "." || name == ".." {
        return Err(TreeError::Outside {
            path: root.join(relative),
        });
    }
    if touches_git(relative) {
        return Err(TreeError::Outside {
            path: root.join(relative),
        });
    }

    let candidate = resolve(root, parent)?.join(name);

    // `symlink_metadata`, not `exists`: `exists` follows the link and asks
    // about its *target*, so a broken symlink — pointing at nothing — reads
    // as absent and would fall through unexamined. A name is "already there"
    // the moment something sits at it, valid or not, and only `resolve`'s own
    // canonicalise-then-check on the whole path can vouch for what it is. A
    // broken link fails that canonicalisation and is refused, correctly: the
    // name is taken by something this cannot vouch for.
    if candidate.symlink_metadata().is_ok() {
        return resolve(root, relative);
    }

    Ok(candidate)
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
/// `from` is resolved with `resolve`, not `resolve_new`: there is nothing to
/// move that is not already there. `to` is refused when it is already taken,
/// the same check `create` makes for the same reason.
pub fn move_to(root: &Path, from: &str, to: &str) -> Result<(), TreeError> {
    let source = resolve(root, from)?;
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
/// `resolve`, not `resolve_new`: there is nothing to delete that is not
/// already there, and `resolve` is what requires it to exist.
pub fn remove(root: &Path, relative: &str) -> Result<(), TreeError> {
    let target = resolve(root, relative)?;
    let removed = if target.is_dir() {
        std::fs::remove_dir_all(&target)
    } else {
        std::fs::remove_file(&target)
    };
    removed.map_err(|source| TreeError::Unreadable {
        path: target,
        source,
    })
}
