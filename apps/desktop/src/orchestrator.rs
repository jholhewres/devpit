//! `orchestrator.open` — the chat that sees every project, one per profile.
//!
//! An orchestrator is a project whose folder is devpit's own, under
//! `orchestrator/<profile>`. Being a project is what gives it the chat, the
//! tabs, the files and the MCP's scope for nothing; being known by its folder
//! is what spares the store a column that would say the same thing.

use std::path::Path;

use devpit_core::Store;
use devpit_rpc::{ErrorCode, Project, RpcError};

/// devpit's half of the brief: rewritten on every opening, so an orchestrator
/// made by an older build learns how this one works. `CLAUDE.md` imports it.
const DEVPIT_BRIEF: (&str, &str) = (
    ".devpit/orchestrator.md",
    include_str!("orchestrator_devpit.md"),
);

/// What a new orchestrator's folder starts with. Written only when missing:
/// the person edits these, and a second opening must not undo that.
const SEEDED: &[(&str, &str)] = &[
    ("CLAUDE.md", include_str!("orchestrator_claude.md")),
    // Where it keeps what it learns: an orchestrator's notes, not a
    // repository — there is no remote to push them to, so no git either.
    ("context/projects/.keep", ""),
    ("decisions/.keep", ""),
    ("docs/.keep", ""),
    ("artifacts/.keep", ""),
];

/// `orchestrator.create` — a new orchestrator speaking as this profile.
///
/// Several may share a profile: each is its own folder, brief and notes, and
/// all of them reach the same sessions, since those belong to the account.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_create(profile_id: String, name: String) -> Result<Project, RpcError> {
    crate::off_main::blocking(move || orchestrator_create_now(profile_id, name)).await
}

/// [`orchestrator_create`], on the calling thread.
pub(crate) fn orchestrator_create_now(
    profile_id: String,
    name: String,
) -> Result<Project, RpcError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 60 {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "an orchestrator needs a name of up to 60 characters",
        ));
    }
    let store = crate::projects::store()?;
    let profile = orchestrable(&store, &profile_id)?;
    let root = Store::root().map_err(|err| RpcError::internal(err.to_string()))?;
    let folder = free_folder(&root, name)
        .ok_or_else(|| RpcError::new(ErrorCode::Invalid, "that name cannot name a folder"))?;
    seed(&folder).map_err(|err| RpcError::internal(err.to_string()))?;
    speaks_as(&folder, &profile.id).map_err(|err| RpcError::internal(err.to_string()))?;
    let here = folder
        .canonicalize()
        .map_err(|err| RpcError::internal(err.to_string()))?;
    let made = crate::projects::project_add_now(here.display().to_string())?;
    let named = crate::project_naming::project_edit_now(
        made.id.clone(),
        name.to_owned(),
        None,
        None,
        None,
    )?;
    Ok(named
        .projects
        .into_iter()
        .find(|one| one.id == made.id)
        .unwrap_or(made))
}

/// `orchestrator.refresh` — devpit's half of an orchestrator's brief, brought
/// up to this build as it is opened.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_refresh(project_id: String) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        seed(&root).map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}

/// The folders an orchestrator reads beyond its own: every project's. Nothing
/// for a project, which stays inside itself.
pub(crate) fn reaches(project_id: &str) -> Vec<String> {
    let Ok(listed) = crate::projects::project_list_now() else {
        return Vec::new();
    };
    let is_orchestrator = listed
        .projects
        .iter()
        .any(|one| one.id == project_id && one.orchestrator.is_some());
    if !is_orchestrator {
        return Vec::new();
    }
    listed
        .projects
        .into_iter()
        .filter(|one| one.orchestrator.is_none())
        .map(|one| one.root_path)
        .collect()
}

/// The profile, if an orchestrator can speak as it: Claude Code, since only it
/// reaches its other sessions, and one devpit can start rather than a shell
/// function it has not read yet.
fn orchestrable(
    store: &devpit_core::Store,
    profile_id: &str,
) -> Result<devpit_rpc::Profile, RpcError> {
    let profile = crate::agent_profiles::all(store)?
        .into_iter()
        .find(|one| one.id == profile_id)
        .ok_or_else(|| {
            RpcError::new(
                ErrorCode::NotFound,
                format!("no profile called {profile_id}"),
            )
        })?;
    if profile.driver != "claude" {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            "an orchestrator runs on Claude Code",
        ));
    }
    if profile.path.is_none() {
        return Err(RpcError::new(
            ErrorCode::Invalid,
            format!(
                "devpit has not read what {} runs yet — open it in Settings → Providers",
                profile.command
            ),
        ));
    }
    Ok(profile)
}

/// Which account an orchestrator speaks as, said in its own folder.
pub(crate) fn speaks_as(folder: &Path, profile_id: &str) -> std::io::Result<()> {
    let file = folder.join(devpit_core::home::ORCHESTRATOR_SETTINGS);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        file,
        serde_json::json!({ "profile": profile_id }).to_string(),
    )
}

/// `orchestrator.account` — the account an orchestrator speaks as, changed.
/// Its next turn starts under that account; its conversations so far stay
/// with the one they began with.
#[tauri::command]
#[specta::specta]
pub async fn orchestrator_account(project_id: String, profile_id: String) -> Result<(), RpcError> {
    crate::off_main::blocking(move || {
        let store = crate::projects::store()?;
        let profile = orchestrable(&store, &profile_id)?;
        let (_, root) = crate::projects::locate(&store, &project_id)?;
        let home = Store::root()
            .ok()
            .and_then(|home| home.canonicalize().ok())
            .unwrap_or_default();
        if devpit_core::home::orchestrator_of(&home, &root).is_none() {
            return Err(RpcError::new(
                ErrorCode::Invalid,
                "that project is not an orchestrator",
            ));
        }
        speaks_as(&root, &profile.id).map_err(|err| RpcError::internal(err.to_string()))
    })
    .await
}

/// A folder for `name` that nothing is using yet.
pub(crate) fn free_folder(root: &Path, name: &str) -> Option<std::path::PathBuf> {
    let base = slug(name);
    (0..100).find_map(|n| {
        let tried = if n == 0 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        devpit_core::home::orchestrator_dir(root, &tried).filter(|folder| !folder.exists())
    })
}

/// A folder name from a person's name for it. Only the folder is plain; the
/// name they gave is what the rail shows.
pub(crate) fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let kept: String = out.trim_matches('-').chars().take(40).collect();
    let kept = kept.trim_end_matches('-');
    if kept.is_empty() {
        "orchestrator".to_owned()
    } else {
        kept.to_owned()
    }
}

/// Writes devpit's brief as this build has it, and what a new orchestrator
/// starts with, leaving the rest alone.
pub(crate) fn seed(folder: &Path) -> std::io::Result<()> {
    let (name, text) = DEVPIT_BRIEF;
    let brief = folder.join(name);
    if std::fs::read_to_string(&brief).ok().as_deref() != Some(text) {
        std::fs::create_dir_all(brief.parent().unwrap_or(folder))?;
        std::fs::write(&brief, text)?;
    }
    for (name, text) in SEEDED {
        let path = folder.join(name);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "orchestrator_tests.rs"]
mod tests;
