//! Reading a directory, safely.
//!
//! Every path this returns has been resolved through symlinks and then checked
//! for containment in the project root. That order is the whole point: a
//! symlink at `web/src/link` pointing at `/etc` passes a string comparison and
//! fails this one. The process runs terminals and writes files, so reaching it
//! is reaching the machine.

use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum TreeError {
    #[error("{path} is outside the project")]
    Outside { path: PathBuf },

    #[error("could not read {path}: {source}")]
    Unreadable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// One entry, before the contract dresses it.
pub struct Entry {
    pub name: String,
    /// Relative to the root, with `/` separators on every platform.
    pub path: String,
    pub is_dir: bool,
}

/// Directories git never wants us to walk into, and nobody wants to see.
///
/// `.git` is not merely noise: it is thousands of files, and walking it turns
/// opening a project into a visible pause.
const SKIP: [&str; 4] = [".git", "node_modules", "target", ".DS_Store"];

/// Resolves `relative` against `root` and refuses anything that escapes.
///
/// Returns the canonical path, so callers cannot accidentally keep using the
/// unresolved one.
pub fn resolve(root: &Path, relative: &str) -> Result<PathBuf, TreeError> {
    let root = root
        .canonicalize()
        .map_err(|source| TreeError::Unreadable {
            path: root.to_path_buf(),
            source,
        })?;

    let candidate = if relative.is_empty() {
        root.clone()
    } else {
        root.join(relative)
    };

    // Canonicalize first, compare second. A `..` segment or a symlink is only
    // visible after resolution, and comparing the strings before it is exactly
    // the check that looks right and is not.
    let resolved = candidate
        .canonicalize()
        .map_err(|source| TreeError::Unreadable {
            path: candidate.clone(),
            source,
        })?;

    if !resolved.starts_with(&root) {
        return Err(TreeError::Outside { path: resolved });
    }

    Ok(resolved)
}

/// One level of children, sorted directories first then naturally.
///
/// One level rather than the whole tree: a monorepo has hundreds of thousands
/// of files, and the screen only ever draws what is expanded.
pub fn children(root: &Path, relative: &str) -> Result<Vec<Entry>, TreeError> {
    let dir = resolve(root, relative)?;
    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let reading = std::fs::read_dir(&dir).map_err(|source| TreeError::Unreadable {
        path: dir.clone(),
        source,
    })?;

    let mut entries = Vec::new();
    for entry in reading.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if SKIP.contains(&name.as_str()) {
            continue;
        }

        // `file_type` does not follow the link, which is what we want: a
        // symlinked directory is listed, and walking into it is refused by
        // `resolve` on the next call if it points outside.
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        let full = entry.path();
        let Ok(child) = full.strip_prefix(&canonical_root) else {
            continue;
        };

        entries.push(Entry {
            name,
            path: child.to_string_lossy().replace('\\', "/"),
            is_dir: kind.is_dir(),
        });
    }

    // `9`, `99`, `100` — the order a person reads. A plain string sort puts
    // `100` first and the list stops being trustworthy at a glance.
    entries.sort_by(|left, right| {
        right
            .is_dir
            .cmp(&left.is_dir)
            .then_with(|| natural(&left.name, &right.name))
    });

    Ok(entries)
}

/// Compares two names with runs of digits read as numbers.
fn natural(left: &str, right: &str) -> std::cmp::Ordering {
    let mut a = left.chars().peekable();
    let mut b = right.chars().peekable();

    loop {
        match (a.peek().copied(), b.peek().copied()) {
            (None, None) => return std::cmp::Ordering::Equal,
            (None, Some(_)) => return std::cmp::Ordering::Less,
            (Some(_), None) => return std::cmp::Ordering::Greater,
            (Some(x), Some(y)) if x.is_ascii_digit() && y.is_ascii_digit() => {
                let left_number = take_number(&mut a);
                let right_number = take_number(&mut b);
                match left_number.cmp(&right_number) {
                    std::cmp::Ordering::Equal => {}
                    other => return other,
                }
            }
            (Some(x), Some(y)) => {
                a.next();
                b.next();
                match x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase()) {
                    std::cmp::Ordering::Equal => {}
                    other => return other,
                }
            }
        }
    }
}

fn take_number(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> u128 {
    let mut value: u128 = 0;
    while let Some(digit) = chars.peek().and_then(|c| c.to_digit(10)) {
        // Saturating: a filename of four hundred digits is not a number, and
        // it must not be a panic either.
        value = value.saturating_mul(10).saturating_add(u128::from(digit));
        chars.next();
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("src")).expect("create");
        std::fs::create_dir_all(dir.path().join(".git")).expect("create");
        for name in ["a.rs", "item100.rs", "item9.rs", "item99.rs"] {
            std::fs::write(dir.path().join("src").join(name), "").expect("write");
        }
        std::fs::write(dir.path().join("README.md"), "").expect("write");
        dir
    }

    #[test]
    fn directories_come_first_and_numbers_sort_as_numbers() {
        let dir = tree();
        let top = children(dir.path(), "").expect("children");
        assert_eq!(top[0].name, "src", "a directory is not first");

        let src = children(dir.path(), "src").expect("children");
        let names: Vec<_> = src.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["a.rs", "item9.rs", "item99.rs", "item100.rs"]);
    }

    #[test]
    fn the_git_directory_is_never_walked() {
        let dir = tree();
        let top = children(dir.path(), "").expect("children");
        assert!(!top.iter().any(|entry| entry.name == ".git"));
    }

    /// The guard this module exists for.
    #[test]
    fn a_path_that_climbs_out_is_refused() {
        let dir = tree();
        let escape = children(dir.path(), "../..");
        assert!(matches!(escape, Err(TreeError::Outside { .. })), "escaped");
    }

    /// A string comparison passes this and a resolved one does not, which is
    /// why the order in `resolve` is not an implementation detail.
    #[cfg(unix)]
    #[test]
    fn a_symlink_pointing_outside_is_refused() {
        let dir = tree();
        let outside = tempfile::tempdir().expect("tempdir");
        std::os::unix::fs::symlink(outside.path(), dir.path().join("escape")).expect("symlink");

        let followed = children(dir.path(), "escape");
        assert!(
            matches!(followed, Err(TreeError::Outside { .. })),
            "a symlink walked out of the project"
        );
    }
}
