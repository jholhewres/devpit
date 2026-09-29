use serde_json::json;

use super::{keep, policy, McpApps, Page, PAGES};

#[test]
fn a_page_reaches_only_the_hosts_its_server_named() {
    let said = policy(&json!({
        "resourceDomains": ["https://api.atlassian.com", "https://*.atl-paas.net"],
        "connectDomains": ["https://api.atlassian.com", "wss://live.atlassian.com"],
    }));
    assert!(said.starts_with("default-src 'none';"));
    assert!(said
        .contains("script-src 'unsafe-inline' https://api.atlassian.com https://*.atl-paas.net;"));
    assert!(said.contains("connect-src https://api.atlassian.com wss://live.atlassian.com;"));
    assert!(said.contains("frame-src 'none';"));
}

#[test]
fn a_declared_host_cannot_widen_the_policy() {
    let said = policy(&json!({
        "resourceDomains": ["https://ok.dev", "*", "http://plain.dev", "https://x.dev; script-src *", "'unsafe-eval'", "data:"],
        "connectDomains": ["https://evil.dev 'unsafe-eval'"],
    }));
    assert!(said.contains("script-src 'unsafe-inline' https://ok.dev;"));
    assert!(said.contains("connect-src 'none';"));
    assert!(!said.contains("unsafe-eval"));
    assert!(!said.contains("http://plain.dev"));
}

#[test]
fn a_page_that_declared_nothing_reaches_nothing() {
    let said = policy(&serde_json::Value::Null);
    assert!(said.contains("connect-src 'none';"));
    assert!(said.contains("media-src 'none';"));
}

fn page(id: &str, tool: &str) -> Page {
    Page {
        id: id.to_owned(),
        of: ("claude".to_owned(), "/w".to_owned(), tool.to_owned()),
        html: String::new(),
        policy: String::new(),
        bordered: false,
    }
}

#[test]
fn a_page_drawn_again_is_the_one_already_open() {
    let apps = McpApps::default();
    keep(
        &mut apps.pages.lock().unwrap(),
        page("first", "mcp__jira__board"),
    );
    let again = apps.kept(&(
        "claude".to_owned(),
        "/w".to_owned(),
        "mcp__jira__board".to_owned(),
    ));
    assert_eq!(again.map(|one| one.id).as_deref(), Some("first"));
}

#[test]
fn the_page_asked_for_least_recently_is_the_one_that_goes() {
    let apps = McpApps::default();
    for at in 0..PAGES {
        keep(
            &mut apps.pages.lock().unwrap(),
            page(&format!("p{at}"), &format!("tool{at}")),
        );
    }
    // The oldest, asked for again, is no longer the next to go.
    assert!(apps
        .kept(&("claude".to_owned(), "/w".to_owned(), "tool0".to_owned()))
        .is_some());
    keep(&mut apps.pages.lock().unwrap(), page("new", "tool_new"));

    let ids: Vec<String> = apps
        .pages
        .lock()
        .unwrap()
        .iter()
        .map(|one| one.id.clone())
        .collect();
    assert_eq!(ids.len(), PAGES);
    assert!(ids.contains(&"p0".to_owned()));
    assert!(!ids.contains(&"p1".to_owned()));
}
