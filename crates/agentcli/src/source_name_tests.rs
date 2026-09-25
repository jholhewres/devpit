//! What the screen calls the place an agent or a skill came from.

use super::*;

/// What the screen calls the place an agent or a skill came from.
///
/// A second installation of the same CLI is a sibling directory —
/// `~/.claude-2` on the machine this was written on — and naming it on
/// screen is a path showing through where a source name belongs.
#[test]
fn every_spelling_of_the_cli_directory_reads_as_claude() {
    for dir in [
        "/home/me/.claude/agents",
        "/home/me/.claude-2/agents",
        "/home/me/.claude-work/skills",
    ] {
        assert_eq!(source_of(Path::new(dir)), "claude", "{dir}");
    }
}

#[test]
fn a_plugin_keeps_its_own_name() {
    assert_eq!(
        source_of(Path::new("/home/me/.claude/plugins/cache/omc/skills")),
        "omc"
    );
    assert_eq!(source_of(Path::new("/home/me/.devpit/agents")), "yours");
}
