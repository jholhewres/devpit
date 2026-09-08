//! Rewriting the ceilings, tightening only.

use std::path::Path;

use crate::ratchet::CEILINGS;

/// Files at or above this many lines get a ceiling. Below it, the ratchet
/// would be noise: short files churn, and every churn would be a failure.
const WORTH_CAPPING: usize = 120;

/// Ceilings land on a step above the file rather than on its exact size.
///
/// Set to the exact count, every added line failed the build — including the
/// four that come with an import and a call. The step leaves room for ordinary
/// work while still failing a file that has grown a whole size class.
const STEP: usize = 50;

fn ceiling_for(lines: usize) -> usize {
    (lines / STEP + 1) * STEP
}

/// Rewrites `xtask/ceilings.txt` from what the tree measures now.
///
/// The guard fails when a ceiling sits above its file, which is correct and
/// also relentless — every time a file gets shorter, a number has to follow it
/// down. Doing that by hand across the whole list is how a guard earns enough
/// resentment to be deleted, so `cargo xtask ceilings` does it.
///
/// It only ever tightens, and that is enforced here rather than left to
/// `check` running first: a number already on record is kept whenever the file
/// now wants a larger one. Otherwise this command would be the way around the
/// guard it exists to serve.
pub fn reseed(root: &Path) -> std::io::Result<usize> {
    let existing = current_ceilings(CEILINGS);
    let mut lines = vec![
        "# Size ceilings. One \"path limit\" per line.".to_owned(),
        "#".to_owned(),
        "# The ratchet only tightens: a file over its ceiling fails the guard, and so".to_owned(),
        "# does a ceiling above what the file needs. Regenerate with `cargo xtask ceilings`"
            .to_owned(),
        "# once a file has actually got shorter.".to_owned(),
        String::new(),
    ];

    let mut capped = Vec::new();
    for area in ["crates", "apps/desktop/src", "web/src", "xtask/src"] {
        for entry in walkdir::WalkDir::new(root.join(area))
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            if path.components().any(|c| c.as_os_str() == "gen") {
                continue;
            }
            if !path.extension().is_some_and(|e| e == "rs" || e == "tsx") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(path) else {
                continue;
            };
            let count = text.lines().count();
            if count < WORTH_CAPPING {
                continue;
            }
            let relative = path.strip_prefix(root).unwrap_or(path);
            let key = relative.display().to_string();
            // Never upward: a file that grew keeps the number it has, and
            // `check` goes on failing until it is split.
            let wanted = ceiling_for(count);
            let ceiling = existing
                .get(&key)
                .map_or(wanted, |on_record| wanted.min(*on_record));
            capped.push(format!("{key} {ceiling}"));
        }
    }
    capped.sort();

    let total = capped.len();
    lines.extend(capped);
    std::fs::write(root.join("xtask/ceilings.txt"), lines.join("\n") + "\n")?;
    Ok(total)
}

/// The ceilings already on record, by path.
fn current_ceilings(list: &str) -> std::collections::HashMap<String, usize> {
    list.lines()
        .filter_map(|line| {
            let line = line.split('#').next().unwrap_or(line).trim();
            let (path, ceiling) = line.rsplit_once(char::is_whitespace)?;
            Some((path.trim().to_owned(), ceiling.trim().parse().ok()?))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The command must not become the way around the guard.
    #[test]
    fn a_number_already_on_record_is_read_back() {
        let on_record = current_ceilings("crates/fake/src/lib.rs 100\n# a note\n");
        assert_eq!(on_record.get("crates/fake/src/lib.rs"), Some(&100));
        assert_eq!(on_record.len(), 1, "the comment was read as an entry");
    }

    /// A file that grew keeps the number it has, so `check` goes on failing
    /// until it is split rather than until someone edits a text file.
    #[test]
    fn a_file_that_grew_keeps_its_old_ceiling() {
        let on_record = current_ceilings("a.rs 100\n");
        let wanted = ceiling_for(140);
        let kept = on_record.get("a.rs").map_or(wanted, |n| wanted.min(*n));
        assert_eq!(kept, 100);
    }

    /// The step is what stops a four-line edit from failing the build.
    #[test]
    fn a_ceiling_lands_above_the_file_not_on_it() {
        assert_eq!(ceiling_for(151), 200);
        assert_eq!(ceiling_for(199), 200);
        assert_eq!(ceiling_for(200), 250);
        assert!(ceiling_for(120) > 120);
    }

    /// A file that shrank gets the smaller number: that is the tightening.
    #[test]
    fn a_file_that_shrank_gets_the_smaller_ceiling() {
        let on_record = current_ceilings("a.rs 100\n");
        let wanted = ceiling_for(30);
        let kept = on_record.get("a.rs").map_or(wanted, |n| wanted.min(*n));
        assert_eq!(kept, 50);
    }
}
