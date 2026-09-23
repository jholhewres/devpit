use super::*;

fn roots() -> (Vec<PathBuf>, Vec<PathBuf>, PathBuf) {
    (
        vec![PathBuf::from("/w/app")],
        vec![
            PathBuf::from("/h/.claude"),
            PathBuf::from("/h/.claude-work"),
        ],
        PathBuf::from("/h/.devpit"),
    )
}

#[test]
fn a_projects_files_and_the_conversations_are_read() {
    let (projects, installs, home) = roots();
    for ok in [
        "/w/app/src/a.ts",
        "/w/app/.playwright-mcp/shot.jpeg",
        "/h/.devpit/projects/app-1/pasted/p.png",
        "/h/.claude/projects/-w-app/s.jsonl",
        "/h/.claude-work/skills/x/SKILL.md",
    ] {
        assert!(readable(Path::new(ok), &projects, &installs, &home), "{ok}");
    }
}

#[test]
fn a_file_that_holds_a_secret_is_never_read() {
    let (projects, installs, home) = roots();
    for no in [
        "/h/.claude/.credentials.json",
        "/h/.claude/settings.json",
        "/h/.claude/settings.local.json",
        "/h/.claude-work/shell-snapshots/snap.sh",
        "/h/.devpit/account-token",
        "/h/.devpit/hook-auth",
        "/h/.devpit/mcp.json",
        "/w/app/.env",
        "/w/app/deploy/id_ed25519",
        "/w/app/certs/server.key",
    ] {
        assert!(
            !readable(Path::new(no), &projects, &installs, &home),
            "{no}"
        );
    }
}
