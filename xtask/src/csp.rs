//! The guard over the window's Content-Security-Policy.
//!
//! The policy is the one line standing between a string that reached the
//! screen and a process this app can start: devpit runs terminals, so a script
//! injected into the window is not a defaced page, it is a shell. It was
//! `null` until plan 16.
//!
//! Only what the window itself fetches passes through it. `plan_limits` and
//! the account talk to the network from the Rust side through reqwest, so
//! `connect-src` has nothing to say about them — the one loopback origin named
//! there is Tauri's own IPC.

use std::path::Path;

use crate::Finding;

/// What the window may reach, and nothing else.
const ALLOWED: [&str; 1] = ["http://ipc.localhost"];

/// What the dev server adds: Vite serves the window and its HMR socket on the
/// same port (`web/vite.config.ts`).
const ALLOWED_IN_DEV: [&str; 3] = [
    "http://ipc.localhost",
    "http://localhost:17800",
    "ws://localhost:17800",
];

/// Directives that may name nothing at all.
const MUST_BE_NONE: [&str; 1] = ["object-src"];

/// Frames come from one place: the scheme a page of an MCP App is served on
/// (`mcp_apps.rs`), an origin of its own, sandboxed — written the two ways
/// Tauri spells a custom scheme, Linux and macOS, then Windows. Nothing on the
/// network is framed.
const FRAMES: [&str; 2] = ["mcpapp:", "http://mcpapp.localhost"];

pub fn the_csp_forbids_what_the_app_never_needs(root: &Path) -> Vec<Finding> {
    let path = root.join("apps/desktop/tauri.conf.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let Ok(conf) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    let relative = path.strip_prefix(root).unwrap_or(&path).to_path_buf();

    let mut findings = Vec::new();
    for (key, allowed) in [("csp", &ALLOWED[..]), ("devCsp", &ALLOWED_IN_DEV[..])] {
        let line = line_of(&text, key);
        match conf
            .get("app")
            .and_then(|app| app.get("security"))
            .and_then(|security| security.get(key))
            .and_then(serde_json::Value::as_str)
        {
            None => findings.push(Finding {
                file: relative.clone(),
                line,
                what: format!("app.security.{key} is missing: a window with no policy"),
            }),
            Some(policy) => {
                findings.extend(refusals(policy, allowed).into_iter().map(|what| Finding {
                    file: relative.clone(),
                    line,
                    what: format!("app.security.{key}: {what}"),
                }))
            }
        }
    }
    findings
}

/// Every reason one policy is not one this app can stand behind.
///
/// Its own function so the test can hand it a policy instead of editing the
/// config — a rule a test cannot call is a rule the test cannot guard.
fn refusals(policy: &str, allowed: &[&str]) -> Vec<String> {
    let mut said = Vec::new();

    // Where scripts may come from is the one directive that decides whether
    // an injected string can run, and a policy that leaves it unsaid allows
    // everything. It falls back to default-src, as the browser does.
    let sources_of = |wanted: &str| {
        policy.split(';').find_map(|directive| {
            let mut words = directive.split_whitespace();
            (words.next() == Some(wanted)).then(|| words.collect::<Vec<_>>())
        })
    };
    match sources_of("script-src").or_else(|| sources_of("default-src")) {
        None => said.push(
            "neither script-src nor default-src is set, so scripts may come from anywhere"
                .to_owned(),
        ),
        Some(sources) if sources != ["'self'"] => said.push(format!(
            "scripts may come from {} — only 'self' is",
            sources.join(" ")
        )),
        Some(_) => {}
    }
    if policy.contains("unsafe-eval") {
        said.push("'unsafe-eval' turns any injected string into code".to_owned());
    }
    if policy.contains('*') {
        said.push("a wildcard source is every host there is".to_owned());
    }

    for directive in policy.split(';') {
        let mut words = directive.split_whitespace();
        let Some(name) = words.next() else {
            continue;
        };
        let sources: Vec<&str> = words.collect();

        if MUST_BE_NONE.contains(&name) && sources != ["'none'"] {
            said.push(format!(
                "{name} names {} instead of 'none'",
                sources.join(" ")
            ));
        }
        if name == "frame-src" {
            if let Some(other) = sources
                .iter()
                .find(|one| **one != "'none'" && !FRAMES.contains(one))
            {
                said.push(format!(
                    "frame-src names {other}: only MCP App pages are framed"
                ));
            }
            continue;
        }
        for source in sources {
            let remote = source.starts_with("http://")
                || source.starts_with("https://")
                || source.starts_with("ws://")
                || source.starts_with("wss://");
            if remote && !allowed.contains(&source) {
                said.push(format!("{name} reaches {source}"));
            }
            if source == "https:" || source == "http:" {
                said.push(format!("{name} reaches any host over {source}"));
            }
        }
    }
    said
}

/// The line a key sits on, so a failure points at it rather than at the file.
fn line_of(text: &str, key: &str) -> usize {
    text.lines()
        .position(|line| line.contains(&format!("\"{key}\"")))
        .map(|at| at + 1)
        .unwrap_or(1)
}

#[cfg(test)]
#[path = "csp_tests.rs"]
mod tests;
