//! What a release says about itself: its own section of `CHANGELOG.md`.
//!
//! The changelog is written for the people who use devpit, and its header
//! promises the release notes come from it. The release page and the update
//! card both read this.

use std::path::Path;

/// The section under `## <version> …`, up to the next `## `, trimmed. `None`
/// when the changelog has no section for that version.
pub(crate) fn section(changelog: &str, version: &str) -> Option<String> {
    let heading = format!("## {version} ");
    let mut lines = changelog
        .lines()
        .skip_while(|line| !line.starts_with(&heading));
    lines.next()?;
    let body: Vec<&str> = lines.take_while(|line| !line.starts_with("## ")).collect();
    let text = body.join("\n").trim().to_owned();
    (!text.is_empty()).then_some(text)
}

/// The section for `version` in the repository's changelog.
pub fn run(root: &Path, version: &str) -> Result<String, String> {
    let changelog =
        std::fs::read_to_string(root.join("CHANGELOG.md")).map_err(|err| err.to_string())?;
    section(&changelog, version).ok_or_else(|| format!("CHANGELOG.md has no section for {version}"))
}

#[cfg(test)]
mod tests {
    use super::section;

    const CHANGELOG: &str = "# Changelog\n\nIntro.\n\n## 0.2.0 — 2026-10-01\n\n### Fixes\n\n- A thing.\n\n## 0.1.9 — 2026-09-01\n\n- Older.\n";

    #[test]
    fn a_version_gets_its_own_section_and_nothing_after_it() {
        assert_eq!(
            section(CHANGELOG, "0.2.0").as_deref(),
            Some("### Fixes\n\n- A thing.")
        );
        assert_eq!(section(CHANGELOG, "0.1.9").as_deref(), Some("- Older."));
    }

    #[test]
    fn a_version_the_changelog_does_not_name_has_no_notes() {
        assert_eq!(section(CHANGELOG, "0.3.0"), None);
        // `0.1` is not `0.1.9`: the heading is matched whole.
        assert_eq!(section(CHANGELOG, "0.1"), None);
    }
}
