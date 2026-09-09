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
