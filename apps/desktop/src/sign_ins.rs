//! The sign-ins a session runs on: the account's own Claude sign-in, read
//! from its file, and what the project names in `needs.txt` and `.env`, asked
//! of their CLIs. The footer lists them; every start checks them first.

use std::path::{Path, PathBuf};

use devpit_agentcli::cli_config::{read_sign_in, SignInRead};
use devpit_rpc::{AccountSignIn, Declared, NeededSignIn, RpcError, SignIns};

use crate::credentials::{self, Missing};
use crate::installations::Found;

/// The bare CLI, which runs against the directory this process would use.
const BARE: &str = "claude";

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as f64)
        .unwrap_or_default()
}

/// An installation as the footer shows it.
pub(crate) fn account_of(found: &Found, read: SignInRead) -> AccountSignIn {
    AccountSignIn {
        label: if found.profiles.is_empty() {
            "Claude Code".to_owned()
        } else {
            found.profiles.join(", ")
        },
        directory: found.directory.display().to_string(),
        state: read.state,
        until: read.until,
        fix: credentials::login_command(found.said.as_deref()),
    }
}

/// The installation `profile_id` runs against: its own, or for the bare CLI
/// the one this process would use, which `installations` lists first.
fn installation_for<'a>(found: &'a [Found], profile_id: &str) -> Option<&'a Found> {
    found
        .iter()
        .find(|one| one.ids.iter().any(|id| id == profile_id))
        .or_else(|| found.first().filter(|_| profile_id == BARE))
}

/// Every installation with a Claude sign-in to keep. One that talks to
/// another provider through a key has none to lapse.
fn accounts() -> Result<Vec<AccountSignIn>, RpcError> {
    let declared = crate::agent_profiles::declared(&crate::projects::store()?);
    let now = now_ms();
    Ok(crate::installations::found()?
        .iter()
        .filter(|one| crate::spend_history::billed(one, &declared))
        .map(|one| account_of(one, read_sign_in(&one.directory, now)))
        .collect())
}

/// What `profile_id`'s own sign-in lacks, when it is a Claude sign-in.
fn account_missing(profile_id: &str) -> Option<Missing> {
    let declared = crate::agent_profiles::declared(&crate::projects::store().ok()?);
    lacks_in(
        &crate::installations::found().ok()?,
        &declared,
        profile_id,
        now_ms(),
    )
}

/// [`account_missing`], among these installations, as of `now_ms`.
pub(crate) fn lacks_in(
    found: &[Found],
    declared: &[Declared],
    profile_id: &str,
    now_ms: f64,
) -> Option<Missing> {
    let one = installation_for(found, profile_id)
        .filter(|one| crate::spend_history::billed(one, declared))?;
    let account = account_of(one, read_sign_in(&one.directory, now_ms));
    credentials::account_lacks(&account.label, account.state, &account.fix)
}

/// The project's `needs.txt` in devpit's folder for it.
pub(crate) fn needs_file(project_id: &str) -> Option<PathBuf> {
    let store = crate::projects::store().ok()?;
    let root = devpit_core::Store::root().ok()?;
    devpit_core::home::ProjectHome::of(&store, &root, project_id)
        .ok()
        .map(|home| home.needs())
}

/// What a start as `profile_id` in the project would go without: the
/// account's own sign-in first, then what the project needs.
pub(crate) fn before_start(
    profile_id: Option<&str>,
    project_id: &str,
    root: &Path,
) -> Vec<Missing> {
    profile_id
        .and_then(account_missing)
        .into_iter()
        .chain(credentials::before_start(
            root,
            needs_file(project_id).as_deref(),
        ))
        .collect()
}

/// Whether `agent_id` starts Claude Code, whose servers `mcp list` reads.
fn drives_claude(agent_id: &str) -> bool {
    let store = crate::projects::store().ok();
    let profiles = store.and_then(|store| crate::agent_profiles::all(&store).ok());
    match profiles.and_then(|all| all.into_iter().find(|one| one.id == agent_id)) {
        Some(profile) => profile.driver == BARE,
        None => agent_id == BARE,
    }
}

/// The MCP servers a start as `agent_id` in the project needs that the last
/// check found not answering. Read from what is kept, never waited on.
pub(crate) fn failing_at_start(
    agent_id: &str,
    project_id: &str,
    root: &Path,
) -> Vec<devpit_rpc::McpServerHealth> {
    if !drives_claude(agent_id) {
        return Vec::new();
    }
    let directory = crate::installations::found().ok().and_then(|found| {
        installation_for(&found, agent_id).map(|one| one.directory.display().to_string())
    });
    let mut needed = needs_file(project_id)
        .and_then(|file| std::fs::read_to_string(file).ok())
        .map(|text| credentials::declared_servers(&text))
        .unwrap_or_default();
    // The servers the project itself declares are ones its sessions use.
    if let Ok(listed) = crate::mcp::mcp_list_now(Some(project_id.to_owned()), directory) {
        let own = listed
            .servers
            .into_iter()
            .filter(|one| one.scope == "project");
        needed.extend(own.map(|one| one.name));
    }
    if needed.is_empty() {
        return Vec::new();
    }
    crate::mcp_health::kept_or_asked(agent_id, root)
        .map(|health| credentials::failing_needed(&health.servers, &needed))
        .unwrap_or_default()
}

/// `sign_ins` — every account's sign-in, and what the open project needs,
/// checked now.
#[tauri::command]
#[specta::specta]
pub async fn sign_ins(project_id: Option<String>) -> Result<SignIns, RpcError> {
    crate::off_main::blocking(move || {
        let needed = match project_id {
            Some(id) => {
                let root = crate::roots::root_of(&id, None)?;
                let needs = credentials::needs_of(&root, needs_file(&id).as_deref());
                credentials::checked(needs)
                    .into_iter()
                    .map(|(need, holds)| NeededSignIn {
                        what: need.label(),
                        holds,
                        fix: need.missing().command,
                    })
                    .collect()
            }
            None => Vec::new(),
        };
        Ok(SignIns {
            accounts: accounts()?,
            needed,
        })
    })
    .await
}

#[cfg(test)]
#[path = "sign_ins_tests.rs"]
mod tests;
