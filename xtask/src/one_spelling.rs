//! The guard over two files whose names differ only in case.
//!
//! Linux tells `acts.ts` and `Acts.tsx` apart; macOS and Windows do not, and
//! neither does the module resolver standing on them. `import './Acts'` looks
//! for `Acts.ts` first, the filesystem answers with `acts.ts`, and a component
//! quietly becomes the module of plain functions beside it — which renders as
//! `undefined` and reads as a bug anywhere but here.
//!
//! It cost five release attempts: ninety-five frontend tests failing only on a
//! Mac, then `TS1149: File name differs from already included file name only
//! in casing` from the build that would have shipped.
//!
//! So the rule is one spelling per name, and it is checked where every other
//! promise of this tree is checked.

use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use crate::Finding;

pub fn every_name_has_one_spelling(root: &Path) -> Vec<Finding> {
    let Ok(listed) = Command::new("git")
        .args(["ls-files"])
        .current_dir(root)
        .output()
    else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&listed.stdout);
    refusals(text.lines())
}

/// Its own function so a test can hand it a listing rather than a repository.
///
/// Compared without the extension, because `acts.ts` and `Acts.tsx` are the
/// pair that bites: the resolver is asked for a name and tries the extensions
/// itself, so two different extensions are no protection at all.
fn refusals<'a>(paths: impl Iterator<Item = &'a str>) -> Vec<Finding> {
    let mut seen: HashMap<String, String> = HashMap::new();
    let mut said = Vec::new();

    for path in paths {
        let Some(stem) = stem_of(path) else { continue };
        let key = stem.to_lowercase();
        match seen.get(&key) {
            Some(first) if first != &stem => said.push(Finding {
                file: path.into(),
                line: 1,
                what: format!(
                    "`{stem}` and `{first}` are the same name to macOS and Windows; \
                     an import of one can resolve to the other"
                ),
            }),
            Some(_) => {}
            None => {
                seen.insert(key, stem);
            }
        }
    }
    said
}

/// The path without its extension, which is what an import names.
fn stem_of(path: &str) -> Option<String> {
    let file = Path::new(path);
    /* Only where a resolver guesses the extension. A `README.md` beside a
    `readme.md` is a mess, and it is not this mess. */
    let interesting = matches!(
        file.extension().and_then(|it| it.to_str()),
        Some("ts" | "tsx" | "js" | "jsx" | "rs")
    );
    if !interesting {
        return None;
    }
    let without = path.strip_suffix(&format!(
        ".{}",
        file.extension().and_then(|it| it.to_str())?
    ))?;
    /* `acts.test.ts` names `acts.test`, which is its own name and not `acts` —
    a resolver asked for `./acts` never reaches it. */
    Some(without.to_owned())
}

#[cfg(test)]
#[path = "one_spelling_tests.rs"]
mod tests;
