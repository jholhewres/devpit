//! A plugin's files on disk, where something other than devpit may have
//! planted a link, a FIFO or a directory first.
//!
//! One implementation for the data API and for the export of the old
//! `drawing` rows, so the two cannot disagree about what such an entry gets.

use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};

/// Makes `relative` (`/` separated) under `base` a segment at a time.
///
/// Anything already there that is not a real directory is refused rather
/// than followed: `create_dir_all` walks through a link, and what the link
/// points at is not a folder of ours to write into.
pub fn dir_inside(base: &Path, relative: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(base)?;
    let mut at = base.to_path_buf();
    for segment in relative.split('/') {
        if matches!(segment, "" | "." | "..") {
            return Err(std::io::Error::other("not a plain relative folder"));
        }
        at.push(segment);
        match std::fs::symlink_metadata(&at) {
            Ok(meta) if meta.is_dir() => {}
            Ok(_) => {
                let taken = format!("{segment} is not a plain directory");
                return Err(std::io::Error::new(ErrorKind::AlreadyExists, taken));
            }
            Err(err) if err.kind() == ErrorKind::NotFound => std::fs::create_dir(&at)?,
            Err(err) => return Err(err),
        }
    }
    Ok(at)
}

/// Deletes `dir` and everything under it, answering how many regular files
/// were counted there first.
///
/// Neither the count nor `remove_dir_all` follows a link inside: the link
/// goes, what it points at stays.
pub fn remove_dir_counting(dir: &Path) -> std::io::Result<u32> {
    let files = regular_files(dir)?;
    std::fs::remove_dir_all(dir)?;
    Ok(files)
}

fn regular_files(dir: &Path) -> std::io::Result<u32> {
    let mut count = 0u32;
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        // `DirEntry::file_type` does not follow a link.
        let kind = entry.file_type()?;
        if kind.is_dir() {
            count = count.saturating_add(regular_files(&entry.path())?);
        } else if kind.is_file() {
            count = count.saturating_add(1);
        }
    }
    Ok(count)
}

/// At most `ceiling` bytes, or `None` when there are more: one byte past the
/// ceiling is read, so a file too big is refused without being held.
pub fn read_within(reader: impl Read, ceiling: u64) -> std::io::Result<Option<Vec<u8>>> {
    let mut raw = Vec::new();
    reader
        .take(ceiling.saturating_add(1))
        .read_to_end(&mut raw)?;
    Ok((raw.len() as u64 <= ceiling).then_some(raw))
}

/// Whether a regular file at `path` already holds exactly `bytes`.
///
/// A link, a FIFO or a directory never does, and is never opened to find out:
/// opening a FIFO waits for a writer that may never come.
pub fn holds(path: &Path, bytes: &[u8]) -> std::io::Result<bool> {
    let meta = std::fs::symlink_metadata(path)?;
    if !meta.is_file() || meta.len() != bytes.len() as u64 {
        return Ok(false);
    }
    let file = std::fs::File::open(path)?;
    Ok(read_within(file, bytes.len() as u64)?.is_some_and(|had| had == bytes))
}

/// Where a write lands before it is renamed over the file: beside it, and
/// hidden, so a listing never shows it.
pub fn temp_for(dir: &Path, name: &str) -> PathBuf {
    dir.join(format!(".{name}.tmp"))
}

/// Writes `bytes` beside `dir/name`, then renames over it, so a reader sees
/// the old file or the new one and never half of either.
///
/// `create_new` refuses a link or a directory planted at the temp name, and
/// `rename` replaces a link at the target instead of writing through it.
pub fn replace(dir: &Path, name: &str, bytes: &[u8]) -> std::io::Result<()> {
    let temp = temp_for(dir, name);
    // A write that died leaves a file; `create_new` refuses whatever else is there.
    if temp.symlink_metadata().is_ok_and(|meta| meta.is_file()) {
        let _ = std::fs::remove_file(&temp);
    }
    let mut made_temp = false;
    let written = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .and_then(|mut file| {
            made_temp = true;
            file.write_all(bytes)?;
            file.sync_all()
        })
        .and_then(|()| std::fs::rename(&temp, dir.join(name)));
    if written.is_err() && made_temp {
        let _ = std::fs::remove_file(&temp);
    }
    written
}
