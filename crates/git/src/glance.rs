//! A folder's git state at a glance: what a terminal's prompt chips show.
//!
//! Two quick asks, not a full status: the branch, and how much the working
//! tree differs from `HEAD`. A prompt is drawn after every command, and a
//! status walk of a large repository is not something to pay that often.

use std::path::Path;

use crate::invoke::run;
use crate::GitError;

/// Where a folder stands, when it is in a repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Glance {
    /// The branch, or a short commit when `HEAD` is detached.
    pub branch: String,
    pub files: u32,
    pub added: u32,
    pub removed: u32,
}

/// The folder's branch and its difference from `HEAD`, or `None` outside a
/// repository.
pub fn glance(folder: &Path) -> Result<Option<Glance>, GitError> {
    let Ok(branch) = run(folder, &["rev-parse", "--abbrev-ref", "HEAD"]) else {
        return Ok(None);
    };
    let mut branch = branch.trim().to_owned();
    if branch == "HEAD" {
        branch = run(folder, &["rev-parse", "--short", "HEAD"])?
            .trim()
            .to_owned();
    }
    // `autoRefreshIndex` off: a glance must not rewrite the index behind the
    // person's back, and so take a lock their own git may be waiting on.
    let stat = run(
        folder,
        &[
            "-c",
            "diff.autoRefreshIndex=false",
            "diff",
            "--shortstat",
            "HEAD",
        ],
    )
    .unwrap_or_default();
    Ok(Some(Glance {
        branch,
        ..shortstat(&stat)
    }))
}

/// ` 3 files changed, 10 insertions(+), 2 deletions(-)`, read by its words.
pub(crate) fn shortstat(line: &str) -> Glance {
    let mut glance = Glance::default();
    for part in line.split(',') {
        let mut words = part.split_whitespace();
        let (Some(count), Some(what)) = (words.next(), words.next()) else {
            continue;
        };
        let Ok(count) = count.parse() else { continue };
        match what {
            w if w.starts_with("file") => glance.files = count,
            w if w.starts_with("insertion") => glance.added = count,
            w if w.starts_with("deletion") => glance.removed = count,
            _ => {}
        }
    }
    glance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shortstat_is_read_by_its_words() {
        let seen = shortstat(" 3 files changed, 10 insertions(+), 2 deletions(-)\n");
        assert_eq!((seen.files, seen.added, seen.removed), (3, 10, 2));
        let one = shortstat(" 1 file changed, 1 deletion(-)");
        assert_eq!((one.files, one.added, one.removed), (1, 0, 1));
        assert_eq!(shortstat(""), Glance::default());
    }

    #[test]
    fn outside_a_repository_there_is_nothing_to_glance_at() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(glance(dir.path()).expect("no error"), None);
    }
}
