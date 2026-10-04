//! `diagnostics.environment` — what devpit sees of this machine, for a bug
//! report: the shell, the `PATH` children get, where the agent CLI and tmux
//! were found, the locale. No variable values beyond those.

use devpit_rpc::RpcError;

/// The report, as text to paste.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct EnvironmentReport {
    pub text: String,
}

#[tauri::command]
#[specta::specta]
pub async fn diagnostics_environment() -> Result<EnvironmentReport, RpcError> {
    crate::off_main::blocking(|| Ok(EnvironmentReport { text: report() })).await
}

fn report() -> String {
    let asked = devpit_pty::login_path::asked();
    let path = devpit_pty::login_path::search_path();
    let found =
        |name: &str| devpit_agentcli::profile::found(name).unwrap_or("not found".to_owned());
    let var = |name: &str| std::env::var(name).unwrap_or("unset".to_owned());
    lines(&[
        ("devpit", env!("CARGO_PKG_VERSION").to_owned()),
        (
            "system",
            format!("{} {}", std::env::consts::OS, std::env::consts::ARCH),
        ),
        ("shell", asked.shell.clone().unwrap_or("unset".to_owned())),
        (
            "login PATH",
            format!("{} in {} ms", asked.outcome, asked.took_ms),
        ),
        ("PATH used", path.to_string_lossy().into_owned()),
        ("claude", found(devpit_agentcli::PROGRAM)),
        ("tmux", found("tmux")),
        ("LANG", var("LANG")),
        ("LC_ALL", var("LC_ALL")),
        ("LC_CTYPE", var("LC_CTYPE")),
    ])
}

/// `name: value`, one per line.
fn lines(pairs: &[(&str, String)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{name}: {value}\n"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_report_names_what_a_bug_report_needs() {
        let text = report();
        for name in [
            "devpit:",
            "shell:",
            "login PATH:",
            "PATH used:",
            "claude:",
            "tmux:",
            "LANG:",
        ] {
            assert!(text.contains(name), "no {name} in\n{text}");
        }
    }
}
