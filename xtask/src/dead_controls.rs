//! Controls that do nothing, counted and only ever allowed to go down.
//!
//! A button with no handler is worse than no button: it says the product does
//! something it does not, and the person who clicked it does not know whether
//! it worked. The prototype was ported whole, so the tree started with a lot
//! of them. Zero on the first day would have meant deleting the port; a
//! budget per file that can only shrink gets to zero without that.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::Finding;

/// Where the budgets live. One `path count` pair per line.
pub(crate) const BUDGETS: &str = include_str!("../dead-controls.txt");

/// No file may grow a control that does nothing.
pub fn a_control_either_works_or_goes(root: &Path) -> Vec<Finding> {
    budgets_in(BUDGETS, root)
}

fn budgets_in(list: &str, root: &Path) -> Vec<Finding> {
    let allowed = parse(list);
    let mut findings = Vec::new();

    for file in sources(&root.join("web/src")) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let relative = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .into_owned();
        let dead = dead_in(&text);
        let budget = allowed.get(&relative).copied().unwrap_or(0);
        if dead > budget {
            findings.push(Finding {
                file: PathBuf::from(&relative),
                line: 1,
                what: format!(
                    "{dead} control(s) with no handler, {budget} allowed — wire it or drop it"
                ),
            });
        }
    }

    for (relative, count) in &allowed {
        if !root.join(relative).exists() {
            findings.push(Finding {
                file: PathBuf::from(relative),
                line: 1,
                what: format!("has a budget of {count} but no file — drop the entry"),
            });
        }
    }

    findings
}

fn parse(list: &str) -> BTreeMap<String, usize> {
    list.lines()
        .filter_map(|line| {
            let entry = line.split('#').next().unwrap_or(line).trim();
            let (path, count) = entry.rsplit_once(char::is_whitespace)?;
            Some((path.trim().to_owned(), count.trim().parse().ok()?))
        })
        .collect()
}

/// Every `.tsx` under a directory.
pub(crate) fn sources(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            found.extend(sources(&path));
        } else if path.extension().is_some_and(|ext| ext == "tsx") {
            found.push(path);
        }
    }
    found.sort();
    found
}

/// How many `<button` tags in this file carry no handler.
///
/// A tag rather than the whole element: a handler always sits inside the
/// opening tag, so the tag is the whole question and nesting never comes into
/// it. `disabled` counts as wired — a control that says it does nothing is
/// telling the truth.
pub(crate) fn dead_in(text: &str) -> usize {
    text.match_indices("<button")
        .filter(|(at, _)| {
            let tag = tag_at(&text[*at..]);
            !tag.contains("onClick")
                && !tag.contains("onPointerDown")
                && !tag.contains("onMouseDown")
                && !tag.contains("type=\"submit\"")
                && !tag.contains("disabled")
        })
        .count()
}

/// The opening tag, up to the `>` that closes it.
///
/// Not the first `>`. An attribute value is a JSX expression, and a `>` inside
/// one closes nothing: `onClick={() => go()}` has one, and so does
/// `aria-label={n > 0 ? "some" : "none"}`. Ending the tag at either reads a
/// wired button as a dead one — the failure mode that makes a guard
/// distrusted, and one this has now had twice.
///
/// Braces are counted rather than special-casing the arrow, because the arrow
/// was only the first way it happened.
fn tag_at(from: &str) -> &str {
    let bytes = from.as_bytes();
    let mut depth = 0usize;
    for (at, byte) in bytes.iter().enumerate() {
        match byte {
            b'{' => depth += 1,
            b'}' => depth = depth.saturating_sub(1),
            b'>' if depth == 0 => return &from[..at],
            _ => {}
        }
    }
    from
}

/// Rewrites `xtask/dead-controls.txt` from what the tree has now.
///
/// Tightening only: a budget already on record is kept whenever the file now
/// wants a larger one, so this command cannot be used to make room. Returns
/// how many dead controls are left.
pub fn reseed(root: &Path) -> std::io::Result<usize> {
    let existing = parse(BUDGETS);
    let mut lines = vec![
        "# Controls with no handler, per file. One \"path count\" per line.".to_owned(),
        "#".to_owned(),
        "# Only ever goes down. A file not listed here is allowed none at all, which".to_owned(),
        "# is the rule for anything written from here on. Regenerate with".to_owned(),
        "# `cargo xtask controls` once controls have actually been wired.".to_owned(),
        String::new(),
    ];

    let mut left = 0;
    for file in sources(&root.join("web/src")) {
        let Ok(text) = std::fs::read_to_string(&file) else {
            continue;
        };
        let relative = file
            .strip_prefix(root)
            .unwrap_or(&file)
            .to_string_lossy()
            .into_owned();
        let dead = dead_in(&text);
        let budget = existing
            .get(&relative)
            .copied()
            .map_or(dead, |had| had.min(dead));
        if budget > 0 {
            lines.push(format!("{relative} {budget}"));
            left += budget;
        }
    }

    lines.push(String::new());
    std::fs::write(root.join("xtask/dead-controls.txt"), lines.join("\n"))?;
    Ok(left)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(findings: &[Finding]) -> String {
        findings
            .iter()
            .map(|one| format!("{}: {}", one.file.display(), one.what))
            .collect::<Vec<_>>()
            .join("; ")
    }

    fn tree(name: &str, contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let web = dir.path().join("web/src/shell");
        std::fs::create_dir_all(&web).expect("create");
        std::fs::write(web.join(name), contents).expect("write");
        dir
    }

    #[test]
    fn a_button_with_a_handler_is_not_counted() {
        assert_eq!(dead_in(r#"<button onClick={go}>Go</button>"#), 0);
    }

    /// Both ways a `>` has turned up inside an attribute and cut the tag
    /// short. Each one read a working button as dead.
    #[test]
    fn a_greater_than_inside_an_attribute_does_not_end_the_tag() {
        assert_eq!(dead_in(r#"<button onClick={() => go()}>Go</button>"#), 0);
        assert_eq!(
            dead_in(r#"<button aria-label={n > 0 ? "some" : "none"} onClick={go}>Go</button>"#),
            0
        );
        // And the tag still ends where it ends: this one really is dead.
        assert_eq!(
            dead_in(r#"<button aria-label={n > 0 ? "some" : "none"}>Go</button>"#),
            1
        );
    }

    #[test]
    fn a_button_with_nothing_behind_it_is_counted() {
        assert_eq!(dead_in(r#"<button className="chip">Go</button>"#), 1);
    }

    /// A control that says it does nothing is telling the truth.
    #[test]
    fn a_disabled_button_is_not_a_lie() {
        assert_eq!(dead_in(r#"<button disabled>Go</button>"#), 0);
    }

    /// The handler is in the opening tag; what the button wraps is not the
    /// question, and a nested one must be counted on its own.
    #[test]
    fn a_button_wrapping_markup_is_read_by_its_own_tag() {
        let markup = r#"<button onClick={go}><span>x</span></button><button>y</button>"#;
        assert_eq!(dead_in(markup), 1);
    }

    /// A handler is written `onClick={() => go()}`. Ending the tag at the
    /// arrow's `>` reads a wired button as a dead one.
    #[test]
    fn an_arrow_function_does_not_end_the_tag() {
        let markup = "<button\n  onMouseMove={() => at(1)}\n  onClick={() => go()}\n>x</button>";
        assert_eq!(dead_in(markup), 0);
    }

    #[test]
    fn a_file_inside_its_budget_passes() {
        let dir = tree("A.tsx", "<button>x</button>\n<button>y</button>\n");
        let findings = budgets_in("web/src/shell/A.tsx 2\n", dir.path());
        assert!(findings.is_empty(), "{}", report(&findings));
    }

    /// The guard exists to catch exactly this: one more than there was.
    #[test]
    fn one_more_dead_control_than_yesterday_fails() {
        let dir = tree("A.tsx", "<button>x</button>\n<button>y</button>\n");
        let findings = budgets_in("web/src/shell/A.tsx 1\n", dir.path());
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].what.contains("2 control(s)"),
            "{}",
            findings[0].what
        );
    }

    /// A file nobody budgeted for gets zero, which is the rule for anything
    /// written from here on.
    #[test]
    fn a_new_file_gets_no_budget_at_all() {
        let dir = tree("New.tsx", "<button>x</button>\n");
        let findings = budgets_in("", dir.path());
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn a_budget_for_a_file_that_is_gone_is_reported() {
        let dir = tree("A.tsx", "");
        let findings = budgets_in("web/src/shell/Gone.tsx 3\n", dir.path());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].what.contains("no file"), "{}", findings[0].what);
    }
}
