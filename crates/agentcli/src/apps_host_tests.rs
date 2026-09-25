use serde_json::json;

use super::{answered, app_tools, plain};

#[test]
fn a_tool_with_a_page_is_named_the_way_the_model_calls_it() {
    let status = json!({ "mcpServers": [
        { "name": "claude.ai Atlassian Rovo", "tools": [
            { "name": "getJiraIssue", "_meta": { "ui": { "resourceUri": "ui://widget/jira-widget.html", "prefersBorder": true, "csp": { "resourceDomains": ["https://api.atlassian.com"] } } } },
            { "name": "atlassianUserInfo" }
        ] },
        { "name": "plugin:datadog:mcp", "tools": [
            { "name": "get_datadog_metric", "_meta": { "ui/resourceUri": "ui://apps/dataviz" } }
        ] }
    ] });
    let found = app_tools(&status);
    assert_eq!(found.len(), 2);
    assert_eq!(
        found[0].called,
        "mcp__claude_ai_Atlassian_Rovo__getJiraIssue"
    );
    assert_eq!(found[0].uri, "ui://widget/jira-widget.html");
    assert_eq!(
        found[0].csp["resourceDomains"][0],
        "https://api.atlassian.com"
    );
    assert_eq!(
        found[1].called,
        "mcp__plugin_datadog_mcp__get_datadog_metric"
    );
    assert_eq!(plain("a.b c"), "a_b_c");
}

#[test]
fn a_refused_request_says_why() {
    assert_eq!(
        answered(json!({ "subtype": "success", "response": { "ok": 1 } })),
        Ok(json!({ "ok": 1 }))
    );
    assert_eq!(
        answered(json!({ "subtype": "error", "error": "no such server" })),
        Err("no such server".to_owned())
    );
}
