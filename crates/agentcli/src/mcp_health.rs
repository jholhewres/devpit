//! `mcp list`: the CLI connects to every MCP server the account has and says
//! how each answered — the one honest reading of their health, since reading
//! the config files only says what is declared.

use std::path::Path;

use devpit_rpc::{McpServerHealth, McpState};

use crate::{running, AgentError, PROGRAM};

/// Asks the CLI, as `runner`'s account and from `cwd` (project servers count).
pub fn mcp_health(
    runner: Option<&running::Runner>,
    cwd: &Path,
) -> Result<Vec<McpServerHealth>, AgentError> {
    let mut argv = vec![runner
        .map_or(PROGRAM, |one| one.program.as_str())
        .to_owned()];
    argv.extend(runner.map(|one| one.args.clone()).unwrap_or_default());
    argv.extend(["mcp".to_owned(), "list".to_owned()]);
    let output = devpit_pty::host_env::command(&argv[0])
        .args(&argv[1..])
        .current_dir(cwd)
        .envs(runner.map(|one| one.env.clone()).unwrap_or_default())
        .output()
        .map_err(|_| AgentError::NotInstalled)?;
    if !output.status.success() {
        return Err(AgentError::Failed {
            command: argv.join(" "),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        });
    }
    Ok(listed(&String::from_utf8_lossy(&output.stdout)))
}

/// Each `name: target - ✔ Connected` line; everything else is skipped.
pub fn listed(text: &str) -> Vec<McpServerHealth> {
    text.lines().filter_map(line).collect()
}

fn line(line: &str) -> Option<McpServerHealth> {
    let (name, rest) = line.split_once(": ")?;
    // The first ` - ` a mark follows: the CLI's reason may hold one too.
    let at = rest
        .match_indices(" - ")
        .map(|(at, _)| at)
        .find(|at| rest[at + 3..].starts_with(['✔', '✘', '!']))?;
    let (target, said) = (&rest[..at], rest[at + 3..].trim());
    let (state, words) = if let Some(words) = said.strip_prefix('✔') {
        (McpState::Connected, words)
    } else if let Some(words) = said.strip_prefix('✘') {
        (McpState::Failed, words)
    } else if let Some(words) = said.strip_prefix('!') {
        (McpState::NeedsAuth, words)
    } else {
        return None;
    };
    let detail = words
        .split_once(" — ")
        .map(|(_, why)| why.trim().to_owned())
        .filter(|why| !why.is_empty());
    Some(McpServerHealth {
        name: name.trim().to_owned(),
        target: target.trim().to_owned(),
        state,
        detail,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_server_is_read_with_its_state_and_why() {
        let said = "Checking MCP server health…\n\n\
            plugin:devpit:devpit: /home/me/.local/bin/devpit mcp - ✔ Connected\n\
            plugin:github:github: https://api.example.com/mcp/ (HTTP) - ✘ Failed to connect — HTTP 400: bad - request\n\
            figma: https://mcp.example.com/mcp (HTTP) - ! Needs authentication\n";
        let found = listed(said);
        assert_eq!(found.len(), 3);
        assert_eq!(found[0].name, "plugin:devpit:devpit");
        assert_eq!(found[0].state, McpState::Connected);
        assert_eq!(found[1].state, McpState::Failed);
        assert_eq!(found[1].target, "https://api.example.com/mcp/ (HTTP)");
        assert_eq!(found[1].detail.as_deref(), Some("HTTP 400: bad - request"));
        assert_eq!(found[2].state, McpState::NeedsAuth);
        assert_eq!(found[2].detail, None);
    }
}
