use serde_json::json;

use super::policy;

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
