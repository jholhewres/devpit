//! Devflow's skills in Codex: the plugin folder devpit writes, added to
//! Codex as a marketplace and installed from it. That writes Codex's own
//! global configuration, so it runs only on the person's click and their yes
//! to what it will do — never on a launch.

use devpit_core::Store;
use devpit_devflow::plugin::{self, Cli, MARKETPLACE, NAME};
use devpit_rpc::{DevflowCodexInstalled, RpcError};

/// How long one `codex` call is given: one that stops to ask, with nobody at
/// its stdin, would otherwise hold the button forever.
const CODEX_WAITS: std::time::Duration = std::time::Duration::from_secs(60);

/// The two commands, in order: the folder as a marketplace, then the plugin
/// from it.
pub(crate) fn steps(folder: &str) -> [Vec<String>; 2] {
    let words = |all: &[&str]| all.iter().map(|one| (*one).to_owned()).collect();
    [
        words(&["plugin", "marketplace", "add", folder]),
        words(&["plugin", "add", &format!("{NAME}@{MARKETPLACE}")]),
    ]
}

/// Runs `codex` with `args`, and says what went wrong in its own words.
fn codex(args: &[String]) -> Result<(), RpcError> {
    let mut command = devpit_pty::host_env::command("codex");
    command.args(args);
    let what = format!("`codex {}`", args.join(" "));
    let out = crate::claude_plugin::output_within(command, CODEX_WAITS)
        .map_err(|why| RpcError::internal(format!("{what} {why}")))?;
    if out.status.success() {
        return Ok(());
    }
    let said = String::from_utf8_lossy(if out.stderr.is_empty() {
        &out.stdout
    } else {
        &out.stderr
    })
    .trim()
    .to_owned();
    Err(RpcError::internal(format!("{what} failed: {said}")))
}

fn install_now() -> Result<DevflowCodexInstalled, RpcError> {
    let root = Store::root()?;
    let exe = crate::agent_reach::exe()
        .ok_or_else(|| RpcError::internal("devpit could not find its own binary"))?;
    let folder = crate::devflow_launch::folder(&root, Cli::Codex);
    crate::devflow_launch::written(&folder, &plugin::codex(&exe.display().to_string()))
        .ok_or_else(|| RpcError::internal("the plugin folder could not be written"))?;
    let shown = folder.display().to_string();
    for step in steps(&shown) {
        codex(&step)?;
    }
    Ok(DevflowCodexInstalled {
        said: format!(
            "Installed: Codex sessions now have Devflow's skills as $devflow-<stage>, from {shown}."
        ),
        folder: shown,
    })
}

/// `devflow.codex_install` — the person's click, after their yes: Devflow's
/// skills added to Codex for every session it starts.
#[tauri::command]
#[specta::specta]
pub async fn devflow_codex_install() -> Result<DevflowCodexInstalled, RpcError> {
    crate::off_main::blocking(install_now).await
}

#[cfg(test)]
#[path = "devflow_codex_tests.rs"]
mod tests;
