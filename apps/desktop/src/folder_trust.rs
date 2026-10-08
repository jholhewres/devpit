//! A project's folders, marked trusted for every Claude Code account devpit
//! knows: when the project is added, when a worktree is made for it and when
//! devpit starts a session in it.
//!
//! The rule and the careful write are `devpit_agentcli::trust`; this decides
//! which files and when. Never a failure for the caller: a folder left
//! untrusted only means the CLI asks, as it always did.

use std::path::{Path, PathBuf};

/// Marks `folders` trusted in every account's settings file, when the person
/// has not turned it off.
pub(crate) fn trust(folders: &[&Path]) {
    let Ok(store) = crate::projects::store() else {
        return;
    };
    if !crate::agent_choice::trust_on(&store) {
        return;
    }
    let Some(home) = home() else {
        return;
    };
    let keys: Vec<String> = folders
        .iter()
        .map(|folder| {
            folder
                .canonicalize()
                .unwrap_or_else(|_| folder.to_path_buf())
        })
        .map(|folder| devpit_agentcli::trust::key_of(&folder))
        .collect();
    let inherited = std::env::var("CLAUDE_CONFIG_DIR").ok();
    let said = crate::agent_profiles::all(&store)
        .unwrap_or_default()
        .into_iter()
        .filter(|one| one.driver == "claude")
        .map(|one| configured(&devpit_agentcli::running::runner(&one).env));
    for file in files(&home, inherited.as_deref(), said) {
        if let Err(why) = devpit_agentcli::trust::trust_in(&file, &keys) {
            devpit_core::reports::background("folder trust", &why);
        }
    }
}

/// What an environment says `CLAUDE_CONFIG_DIR` is, if it says.
fn configured(env: &[(String, String)]) -> Option<String> {
    env.iter()
        .rev()
        .find(|(name, _)| name == "CLAUDE_CONFIG_DIR")
        .map(|(_, value)| value.clone())
}

/// Each account's settings file, once: the one this process's agents inherit
/// and every profile's own. A file outside `home` is not this person's — it is
/// a test's home borrowing the real environment — and is left alone.
pub(crate) fn files(
    home: &Path,
    inherited: Option<&str>,
    profiles: impl Iterator<Item = Option<String>>,
) -> Vec<PathBuf> {
    let mut files = vec![devpit_agentcli::cli_config::settings_file_from(
        home, inherited,
    )];
    for said in profiles {
        files.push(devpit_agentcli::cli_config::settings_file_from(
            home,
            said.as_deref().or(inherited),
        ));
    }
    let mut once = Vec::new();
    for file in files {
        if file.starts_with(home) && !once.contains(&file) {
            once.push(file);
        }
    }
    once
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
#[path = "folder_trust_tests.rs"]
mod tests;
