//! Finding the profiles this machine can run.

use std::path::PathBuf;

use crate::driver::driver;
pub use devpit_rpc::Profile;

/// Where a command resolves to, if anywhere.
pub fn found(command: &str) -> Option<String> {
    std::env::var_os("PATH").and_then(|path| {
        std::env::split_paths(&path)
            .map(|dir| dir.join(command))
            .find(is_runnable)
            .map(|found| found.to_string_lossy().into_owned())
    })
}

#[cfg(unix)]
fn is_runnable(path: &PathBuf) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_runnable(path: &PathBuf) -> bool {
    path.is_file()
}

/// What the driver offers, or nothing when no driver answers to that name.
fn models_of(name: &str) -> Vec<String> {
    driver(name)
        .map(|found| found.models().iter().map(|&m| m.to_owned()).collect())
        .unwrap_or_default()
}

/// The commands worth looking for when nothing has been declared.
const KNOWN: &[(&str, &str, &str)] = &[
    ("claude", "Claude Code", "claude"),
    ("claudin", "Claude Code (second account)", "claude"),
];

/// Every profile: the declared ones first, then the known commands that are
/// installed and not already named.
pub fn profiles(declared: &[Profile]) -> Vec<Profile> {
    let mut all: Vec<Profile> = declared
        .iter()
        .map(|profile| Profile {
            path: found(&profile.command),
            models: models_of(&profile.driver),
            ..profile.clone()
        })
        .collect();

    for (command, label, driver) in KNOWN {
        if all.iter().any(|profile| profile.command == *command) {
            continue;
        }
        if let Some(path) = found(command) {
            all.push(Profile {
                id: (*command).to_owned(),
                label: (*label).to_owned(),
                command: (*command).to_owned(),
                driver: (*driver).to_owned(),
                path: Some(path),
                models: models_of(driver),
            });
        }
    }
    all
}
