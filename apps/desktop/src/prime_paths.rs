//! Which files a worktree's setup names: paths held to the checkout, and
//! `*` patterns within one name.

use std::path::Path;

/// A declared path as written: relative, and never climbing out.
pub(crate) fn plain(relative: &str) -> bool {
    use std::path::Component;
    !relative.trim().is_empty()
        && Path::new(relative)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
}

/// Whether `name` fits `pattern`, where `*` is any run of characters.
pub(crate) fn fits(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let pieces: Vec<&str> = parts.collect();
    let Some((last, middle)) = pieces.split_last() else {
        return rest.is_empty();
    };
    for piece in middle {
        match rest.find(piece) {
            Some(at) => rest = &rest[at + piece.len()..],
            None => return false,
        }
    }
    rest.len() >= last.len() && rest.ends_with(last)
}

/// The files of `main` a copy pattern names, relative to it.
pub(crate) fn copied(main: &Path, pattern: &str) -> Vec<String> {
    if !plain(pattern) {
        return Vec::new();
    }
    let (folder, name) = pattern.rsplit_once('/').unwrap_or(("", pattern));
    let Ok(entries) = std::fs::read_dir(main.join(folder)) else {
        return Vec::new();
    };
    let mut found: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|one| fits(name, one))
        .map(|one| {
            if folder.is_empty() {
                one
            } else {
                format!("{folder}/{one}")
            }
        })
        .collect();
    found.sort();
    found
}

#[cfg(test)]
#[path = "prime_paths_tests.rs"]
mod tests;
