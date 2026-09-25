//! devpit's own MCP Apps: pages a host shows under devpit's tools — the
//! board, a card, the sessions — built from the same answers the tools give.
//!
//! Each page is one self-contained document: its style and script inline,
//! nothing fetched from anywhere, so its policy grants no host at all. It
//! reaches devpit only by asking the host to call devpit's tools.

use serde_json::{json, Value};

/// The MIME type a host renders as an app.
pub const APP_MIME: &str = "text/html;profile=mcp-app";

const STYLE: &str = include_str!("apps/page.css");
const CLIENT: &str = include_str!("apps/client.js");
const CARD_VIEW: &str = include_str!("apps/cardview.js");

struct Page {
    uri: &'static str,
    name: &'static str,
    /// The tools this page is shown under.
    tools: &'static [&'static str],
    script: &'static [&'static str],
}

const PAGES: [Page; 3] = [
    Page {
        uri: "ui://devpit/board.html",
        name: "devpit board",
        tools: &["devpit_board"],
        script: &[CARD_VIEW, include_str!("apps/board.js")],
    },
    Page {
        uri: "ui://devpit/card.html",
        name: "devpit card",
        tools: &["devpit_card"],
        script: &[CARD_VIEW, include_str!("apps/card.js")],
    },
    Page {
        uri: "ui://devpit/sessions.html",
        name: "devpit sessions",
        tools: &["devpit_sessions"],
        script: &[include_str!("apps/sessions.js")],
    },
];

/// The page a tool is shown with, as its `_meta`.
pub fn meta_for(tool: &str) -> Option<Value> {
    PAGES
        .iter()
        .find(|page| page.tools.contains(&tool))
        .map(|page| json!({ "ui": { "resourceUri": page.uri, "prefersBorder": true }, "ui/resourceUri": page.uri }))
}

/// `resources/list`: the pages.
pub fn listed() -> Value {
    json!({ "resources": PAGES.iter().map(|page| json!({
        "uri": page.uri,
        "name": page.name,
        "mimeType": APP_MIME,
    })).collect::<Vec<_>>() })
}

/// `resources/read`: one page, whole.
pub fn read(uri: &str) -> Option<Value> {
    let page = PAGES.iter().find(|page| page.uri == uri)?;
    let html = format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\">\
         <style>{STYLE}</style></head><body><div id=\"app\"><p class=\"muted\">Loading…</p></div>\
         <script>{CLIENT}\n{}</script></body></html>",
        page.script.join("\n")
    );
    Some(json!({ "contents": [{
        "uri": page.uri,
        "mimeType": APP_MIME,
        "text": html,
        // Nothing from outside: every host is left out.
        "_meta": { "ui": { "csp": { "resourceDomains": [], "connectDomains": [] }, "prefersBorder": true } },
    }] }))
}
