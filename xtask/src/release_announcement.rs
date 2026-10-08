//! What the community's `#releases` channel says when a version is published:
//! the version, a few highlights from its changelog section, and the link.
//!
//! Printed as a Discord webhook body, so the workflow only has to post it and
//! a person can read exactly what would be posted before it is.

use std::path::Path;

use crate::release_notes;

/// Enough to say what a release is about without becoming the changelog.
const MOST: usize = 5;
/// devpit's accent, the colour the embed's edge is drawn in.
const ACCENT: u32 = 0xe2_79_5b;

/// The highlights of a changelog section: the bold lead of each top-level
/// bullet, first one per `###` group, then the rest in order, up to five.
pub(crate) fn highlights(section: &str) -> Vec<String> {
    let mut groups: Vec<Vec<String>> = vec![Vec::new()];
    for line in section.lines() {
        if line.starts_with("### ") {
            groups.push(Vec::new());
        } else if let Some(bullet) = line.strip_prefix("- ") {
            if let Some(group) = groups.last_mut() {
                group.push(lead(bullet));
            }
        }
    }
    let groups: Vec<Vec<String>> = groups.into_iter().filter(|g| !g.is_empty()).collect();
    let mut picked: Vec<String> = groups.iter().map(|g| g[0].clone()).collect();
    picked.extend(groups.iter().flat_map(|g| g.iter().skip(1).cloned()));
    picked.truncate(MOST);
    picked
}

/// `**A thing works.** More words` → `A thing works.`; a bullet with no bold
/// lead keeps its first line.
fn lead(bullet: &str) -> String {
    bullet
        .strip_prefix("**")
        .and_then(|rest| rest.split_once("**"))
        .map_or(bullet, |(bold, _)| bold)
        .trim()
        .to_owned()
}

/// The webhook body for `version`, linking to `url`.
pub(crate) fn body(version: &str, section: &str, url: &str) -> serde_json::Value {
    let lines: Vec<String> = highlights(section)
        .iter()
        .map(|one| format!("• {one}"))
        .collect();
    serde_json::json!({
        "allowed_mentions": { "parse": [] },
        "embeds": [{
            "title": format!("devpit {version}"),
            "url": url,
            "description": format!("{}\n\n[Release notes and downloads]({url})", lines.join("\n")),
            "color": ACCENT,
        }],
    })
}

/// The body for `version` from the repository's changelog.
pub fn run(root: &Path, version: &str, url: &str) -> Result<String, String> {
    let section = release_notes::run(root, version)?;
    Ok(body(version, &section, url).to_string())
}

#[cfg(test)]
mod tests {
    use super::{body, highlights};

    const SECTION: &str = "### Sessions\n\n- **A command can be sent.** And more.\n- **Ended ones stay.** Text.\n  continued line\n\n### Files\n\n- **Video plays.** In a tab.\n- Plain bullet with no lead\n\n### Settings\n\n- **Voice tests.** Here.\n\n### Remote\n\n- **Key bar.** Yes.\n\n### Board\n\n- **Columns.** Fine.\n- **Cards.** Too.";

    #[test]
    fn each_group_leads_with_its_first_bullet_before_any_group_gets_a_second() {
        assert_eq!(
            highlights(SECTION),
            vec![
                "A command can be sent.",
                "Video plays.",
                "Voice tests.",
                "Key bar.",
                "Columns.",
            ]
        );
    }

    #[test]
    fn a_short_section_fills_with_the_rest_in_order() {
        let short = "### One\n\n- **First.** a\n- **Second.** b\n- Third, unbolded\n";
        assert_eq!(
            highlights(short),
            vec!["First.", "Second.", "Third, unbolded"]
        );
    }

    #[test]
    fn the_post_names_the_version_links_the_release_and_pings_nobody() {
        let posted = body("0.2.0", "- **A thing.** b", "https://example.test/v0.2.0");
        let embed = &posted["embeds"][0];
        assert_eq!(embed["title"], "devpit 0.2.0");
        assert_eq!(embed["url"], "https://example.test/v0.2.0");
        let description = embed["description"].as_str().unwrap_or_default();
        assert!(description.starts_with("• A thing."));
        assert!(description.contains("(https://example.test/v0.2.0)"));
        assert_eq!(posted["allowed_mentions"]["parse"], serde_json::json!([]));
    }
}
