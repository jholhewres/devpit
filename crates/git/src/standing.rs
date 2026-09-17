//! Where a working tree stands, in one short string.
//!
//! A result from a run is about the code that run saw. To say later whether it
//! is still about the code in front of somebody, the two have to be
//! comparable — and the comparison has to include uncommitted work, because
//! uncommitted work is most of what a person is looking at while they are
//! working.
//!
//! So: the commit, plus a digest of every changed path and the size of its
//! edit. Hashed rather than kept whole because a repository mid-refactor has
//! hundreds of changed files, and a run row is not where that belongs.
//!
//! **What this does not prove.** Two matching strings mean git sees the same
//! tracked state. They do not mean the run was isolated: an ignored file, an
//! installed package, an environment variable and the clock are all outside
//! this and all able to change an answer. Nothing built on it may claim more.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::status::changes;
use crate::GitError;

/// A digest of everything uncommitted, or an error when git will not say.
///
/// Stable across runs for the same tree: `changes` reads a status ordered by
/// path, and the digest is taken over that order.
pub fn standing_at(root: &Path) -> Result<String, GitError> {
    let changed = changes(root)?;
    let mut digest = Sha256::new();
    for change in &changed {
        // The size of the edit, not its content: a file edited and put back is
        // the same tree, and reading every changed file to notice that would
        // cost more than the question is worth.
        digest.update(change.path.as_bytes());
        digest.update([0]);
        digest
            .update(format!("{:?}:{}:{}", change.status, change.added, change.removed).as_bytes());
        digest.update([0]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

#[cfg(test)]
#[path = "standing_tests.rs"]
mod tests;
