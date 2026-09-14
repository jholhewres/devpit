use super::*;

const CONFIG: &str = r#"{
  "mcpServers": {
    "playwright": { "command": "npx", "args": ["@playwright/mcp@latest"] },
    "anchored": { "url": "https://anchored.example/mcp" }
  }
}"#;

#[test]
fn a_server_reached_by_a_command_shows_the_whole_line() {
    let found = servers_in(CONFIG, "user");
    let playwright = found
        .iter()
        .find(|one| one.name == "playwright")
        .expect("found");
    assert_eq!(playwright.reached_by, "npx @playwright/mcp@latest");
    assert_eq!(playwright.scope, "user");
}

#[test]
fn a_server_reached_by_a_url_shows_the_url() {
    let found = servers_in(CONFIG, "project");
    let anchored = found
        .iter()
        .find(|one| one.name == "anchored")
        .expect("found");
    assert_eq!(anchored.reached_by, "https://anchored.example/mcp");
}

#[test]
fn a_config_with_no_servers_gives_none_rather_than_failing() {
    assert!(servers_in(r#"{"other": 1}"#, "user").is_empty());
}

/// The CLI's config is not ours; a shape we cannot read must not take the
/// panel down with it.
#[test]
fn a_config_that_is_not_json_gives_none_rather_than_failing() {
    assert!(servers_in("not json at all", "user").is_empty());
}

#[test]
fn the_names_come_back_sorted_so_the_panel_does_not_reshuffle() {
    let names: Vec<String> = servers_in(CONFIG, "user")
        .into_iter()
        .map(|one| one.name)
        .collect();
    assert_eq!(names, vec!["anchored".to_owned(), "playwright".to_owned()]);
}

/// What `~/.claude.json` holds per working directory.
///
/// The comment above `servers_in` used to claim this shape was accepted; it
/// was not. Measured on one machine: five servers across five projects, none
/// of them ever reaching the panel.
const SETTINGS: &str = r#"{
  "mcpServers": { "everywhere": { "command": "one" } },
  "projects": {
    "/home/someone/work/api": { "mcpServers": { "reports": { "command": "two" } } },
    "/home/someone/work/web": { "mcpServers": { "laravel-boost": { "command": "three" } } }
  }
}"#;

#[test]
fn a_projects_own_servers_are_found_under_its_path() {
    let found = servers_for(SETTINGS, "/home/someone/work/api", "project");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "reports");
    assert_eq!(found[0].reached_by, "two");
    assert_eq!(found[0].scope, "project");
}

#[test]
fn another_projects_servers_are_not_this_projects() {
    // Keyed by the directory the CLI was started in, so a near miss on the
    // path has to give nothing rather than the neighbour's list.
    assert!(servers_for(SETTINGS, "/home/someone/work", "project").is_empty());
    assert!(servers_for(SETTINGS, "/home/someone/work/api/sub", "project").is_empty());
}

#[test]
fn a_settings_file_with_no_projects_gives_none_rather_than_failing() {
    assert!(servers_for(r#"{"mcpServers":{"a":{}}}"#, "/x", "project").is_empty());
    assert!(servers_for("not json at all", "/x", "project").is_empty());
}

#[test]
fn the_user_wide_servers_are_still_read_from_the_top_level() {
    let found = servers_in(SETTINGS, "user");
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "everywhere");
}
