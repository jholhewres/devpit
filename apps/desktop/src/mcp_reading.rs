//! The two shapes an MCP server is written in, and what to call it.
//!
//! Apart from `mcp.rs` because that file is the command — which files to read,
//! in which order, and what to say when there is nothing. This one is the
//! grammar, which is the part with cases in it.

use crate::mcp::Server;

/// The servers named at the top level of a config file.
///
/// `{"mcpServers": {...}}` as the file's whole content, which is `.mcp.json`
/// and the user-wide section of `~/.claude.json`.
pub(crate) fn servers_in(text: &str, scope: &str) -> Vec<Server> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    named(value.get("mcpServers"), scope)
}

/// The servers the CLI keeps for one working directory.
///
/// `~/.claude.json` holds these under `projects.<absolute path>.mcpServers`,
/// keyed by the directory the CLI was started in. Nothing read them before,
/// while the comment above `servers_in` claimed they were — measured on one
/// machine, five servers across five projects, none of them ever shown.
pub(crate) fn servers_for(text: &str, root: &str, scope: &str) -> Vec<Server> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return Vec::new();
    };
    named(
        value
            .get("projects")
            .and_then(|projects| projects.get(root))
            .and_then(|project| project.get("mcpServers")),
        scope,
    )
}

/// One `mcpServers` object as rows, sorted so the panel does not reshuffle.
fn named(found: Option<&serde_json::Value>, scope: &str) -> Vec<Server> {
    let Some(map) = found.and_then(|found| found.as_object()) else {
        return Vec::new();
    };
    let mut servers: Vec<Server> = map
        .iter()
        .map(|(name, config)| Server {
            name: name.clone(),
            scope: scope.to_owned(),
            reached_by: reached_by(config),
        })
        .collect();
    servers.sort_by(|a, b| a.name.cmp(&b.name));
    servers
}

/// How a server is reached, in one line, with what looks like a secret masked:
/// a config often carries its token in the URL or on the command line, and the
/// panel is on screen.
fn reached_by(config: &serde_json::Value) -> String {
    if let Some(url) = config.get("url").and_then(|found| found.as_str()) {
        return masked_url(url);
    }
    let command = config
        .get("command")
        .and_then(|found| found.as_str())
        .unwrap_or_default();
    let args: Vec<&str> = config
        .get("args")
        .and_then(|found| found.as_array())
        .map(|list| list.iter().filter_map(|one| one.as_str()).collect())
        .unwrap_or_default();
    if args.is_empty() {
        command.to_owned()
    } else {
        format!("{command} {}", masked_args(&args).join(" "))
    }
}

const MASK: &str = "***";

/// Words that make a name a credential: `--api-key`, `--token`, `API_KEY=`.
const SECRET_WORDS: [&str; 8] = [
    "key",
    "token",
    "secret",
    "password",
    "pass",
    "auth",
    "bearer",
    "credential",
];

fn secret_name(name: &str) -> bool {
    let name = name.trim_start_matches('-').to_ascii_lowercase();
    SECRET_WORDS.iter().any(|word| name.contains(word))
}

/// A flag whose value is a credential by its name, or one that carries a
/// header or an environment variable — `-H "Authorization: …"`, `-e TOKEN=…`
/// — whatever the header or variable is called.
fn secret_flag(flag: &str) -> bool {
    flag.starts_with('-')
        && (secret_name(flag)
            || ["-h", "--header", "-e", "--env"].contains(&flag.to_ascii_lowercase().as_str()))
}

/// An argument that is a credential on its own: an `Authorization:` header
/// written as one word, or `NAME=value` where the name says what it holds.
fn secret_word(arg: &str) -> bool {
    let lower = arg.to_ascii_lowercase();
    lower.starts_with("authorization:")
        || lower.starts_with("bearer ")
        || arg.split_once('=').is_some_and(|(name, _)| {
            !name.starts_with('-') && !name.contains("://") && secret_name(name)
        })
}

/// The arguments with a secret flag's value masked, in both `--flag value` and
/// `--flag=value`, and every URL among them masked as [`masked_url`] does.
fn masked_args(args: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(args.len());
    let mut hides_next = false;
    for arg in args {
        if hides_next {
            hides_next = false;
            out.push(MASK.to_owned());
            continue;
        }
        match arg.split_once('=') {
            Some((flag, _)) if secret_flag(flag) => out.push(format!("{flag}={MASK}")),
            _ if secret_word(arg) => out.push(match arg.split_once(['=', ':']) {
                Some((name, _)) => format!("{name}={MASK}"),
                None => MASK.to_owned(),
            }),
            None if secret_flag(arg) => {
                hides_next = true;
                out.push((*arg).to_owned());
            }
            _ if arg.contains("://") => out.push(masked_url(arg)),
            _ => out.push((*arg).to_owned()),
        }
    }
    out
}

/// A path segment that reads as a key: long, and nothing but key characters.
/// Hosted servers put the secret there — `/api/mcp/s/<secret>/mcp`.
fn secret_segment(segment: &str) -> bool {
    segment.len() >= 24
        && segment
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        && segment.chars().any(|c| c.is_ascii_digit())
}

/// The URL with its user information, its query values and its fragment
/// masked: the places a token rides in a URL.
fn masked_url(url: &str) -> String {
    let (url, fragment) = match url.split_once('#') {
        Some((url, _)) => (url, Some(MASK)),
        None => (url, None),
    };
    let (url, query) = match url.split_once('?') {
        Some((url, query)) => (url, Some(query)),
        None => (url, None),
    };
    let mut out = match url.split_once("://") {
        Some((scheme, rest)) => {
            let end = rest.find('/').unwrap_or(rest.len());
            let (authority, path) = rest.split_at(end);
            let path: Vec<&str> = path
                .split('/')
                .map(|segment| {
                    if secret_segment(segment) {
                        MASK
                    } else {
                        segment
                    }
                })
                .collect();
            let path = path.join("/");
            match authority.rsplit_once('@') {
                Some((_, host)) => format!("{scheme}://{MASK}@{host}{path}"),
                None => format!("{scheme}://{authority}{path}"),
            }
        }
        None => url.to_owned(),
    };
    if let Some(query) = query {
        let pairs: Vec<String> = query
            .split('&')
            .map(|pair| match pair.split_once('=') {
                Some((name, _)) => format!("{name}={MASK}"),
                None => pair.to_owned(),
            })
            .collect();
        out.push('?');
        out.push_str(&pairs.join("&"));
    }
    if let Some(fragment) = fragment {
        out.push('#');
        out.push_str(fragment);
    }
    out
}

#[cfg(test)]
#[path = "mcp_reading_tests.rs"]
mod tests;
