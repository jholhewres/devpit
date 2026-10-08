//! A project's new-worktree setup, read and written from the window.

use crate::prime::Prime;
use crate::prime_paths::plain;

/// What a project declared, or — when it declared nothing at all — its
/// `.env*` files copied: a worktree without them starts a session broken, and
/// a tracked one is already there and left as it is.
pub(crate) fn declared_or_env(path: &std::path::Path) -> Prime {
    if path.exists() {
        return crate::prime::read(path);
    }
    Prime {
        copy: vec![".env*".to_owned()],
        ..Prime::default()
    }
}

/// `project.worktree_setup` — what a new worktree of this project gets.
#[tauri::command]
#[specta::specta]
pub async fn project_worktree_setup(
    project_id: String,
) -> Result<devpit_rpc::WorktreeSetup, devpit_rpc::RpcError> {
    crate::off_main::blocking(move || {
        let path = crate::projects::project_home(&project_id)?.prime();
        Ok(shown(declared_or_env(&path)))
    })
    .await
}

/// `project.worktree_setup_set` — the same, written.
#[tauri::command]
#[specta::specta]
pub async fn project_worktree_setup_set(
    project_id: String,
    setup: devpit_rpc::WorktreeSetup,
) -> Result<devpit_rpc::WorktreeSetup, devpit_rpc::RpcError> {
    crate::off_main::blocking(move || {
        let path = crate::projects::project_home(&project_id)?.prime();
        let mut prime = kept(setup);
        // Not on the screen, so a save keeps whatever the file said.
        prime.timeout = crate::prime::read(&path).timeout;
        let text = serde_json::to_string_pretty(&prime)
            .map_err(|err| devpit_rpc::RpcError::internal(err.to_string()))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| devpit_rpc::RpcError::internal(err.to_string()))?;
        }
        std::fs::write(&path, text)
            .map_err(|err| devpit_rpc::RpcError::internal(err.to_string()))?;
        Ok(shown(prime))
    })
    .await
}

fn shown(prime: Prime) -> devpit_rpc::WorktreeSetup {
    devpit_rpc::WorktreeSetup {
        copy: prime.copy,
        link: prime.link,
        run: prime.run,
        share: prime
            .share
            .into_iter()
            .map(|(name, value)| devpit_rpc::EnvVar { name, value })
            .collect(),
    }
}

/// What the screen sent, as it is kept: blank lines dropped, and a path that
/// would climb out of the checkout refused by leaving it out.
fn kept(setup: devpit_rpc::WorktreeSetup) -> Prime {
    let lines = |all: Vec<String>| -> Vec<String> {
        all.into_iter()
            .map(|one| one.trim().to_owned())
            .filter(|one| !one.is_empty())
            .collect()
    };
    Prime {
        copy: lines(setup.copy)
            .into_iter()
            .filter(|one| plain(one))
            .collect(),
        link: lines(setup.link)
            .into_iter()
            .filter(|one| plain(one))
            .collect(),
        run: lines(setup.run),
        share: setup
            .share
            .into_iter()
            .filter(|one| !one.name.trim().is_empty())
            .map(|one| (one.name.trim().to_owned(), one.value))
            .collect(),
        timeout: None,
    }
}

#[cfg(test)]
#[path = "worktree_setup_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "prime_defaults_tests.rs"]
mod defaults_tests;
