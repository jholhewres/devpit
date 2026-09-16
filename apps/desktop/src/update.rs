//! The rules an update follows, and the check that uses them.
//!
//! Three of them are pure, because they are the ones worth arguing about: what
//! may happen next (`next`), when to look again (`due`), and what this copy of
//! devpit even is (`install_kind`). Everything else here is plumbing around
//! the updater plugin.

use devpit_rpc::{InstallKind, RpcError, UpdateStatus};

/// What can happen to an update, from the app or from the network.
///
/// One variant per thing that actually happens in this build: downloading,
/// waiting and installing arrive with the code that raises them, so nothing
/// here is a state nobody can reach.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event {
    /// Someone asked, or the timer did.
    Check,
    Found {
        version: String,
        notes: String,
        kind: InstallKind,
    },
    /// Checked, and this is the newest there is.
    NothingNewer,
    Failed {
        message: String,
    },
}

/// The next state, or `None` when the event is refused.
///
/// Refusal is the point: checking while a download runs, installing twice, or
/// anything at all once the installer has started are the three ways a person
/// clicking quickly turns one update into two.
pub(crate) fn next(state: &UpdateStatus, event: Event) -> Option<UpdateStatus> {
    use Event as E;
    use UpdateStatus as S;

    match (state, event) {
        // Past the point of return. Nothing but its own failure moves it.
        (S::Installing, E::Failed { message }) => Some(S::Failed {
            message,
            recoverable: false,
        }),
        (S::Installing, _) => None,

        (S::Idle | S::Failed { .. } | S::Available { .. }, E::Check) => Some(S::Checking),
        (
            S::Checking,
            E::Found {
                version,
                notes,
                kind,
            },
        ) => Some(match kind {
            InstallKind::ExternallyManaged => S::ExternallyManaged,
            kind => S::Available {
                version,
                notes,
                kind,
            },
        }),
        (S::Checking, E::NothingNewer) => Some(S::Idle),
        (S::Checking | S::Downloading { .. }, E::Failed { message }) => Some(S::Failed {
            message,
            recoverable: true,
        }),

        // A check while something is in flight is refused here, once, rather
        // than politely somewhere else.
        _ => None,
    }
}

/// What this copy of devpit is, from what the machine says about it.
///
/// `appimage` is `$APPIMAGE` when the running binary is one; `bundle_type` is
/// the stamp the bundler wrote; `tools` are the package tools actually found
/// in the fixed system directories — never from `$PATH`, which is somebody
/// else's to write.
pub(crate) fn install_kind(
    appimage: Option<&str>,
    bundle_type: Option<&str>,
    tools: &[String],
) -> InstallKind {
    if appimage.is_some_and(|path| !path.is_empty()) {
        return InstallKind::AppImage;
    }
    match bundle_type {
        Some("deb") | Some("rpm") => {
            if tools.is_empty() {
                InstallKind::ExternallyManaged
            } else {
                InstallKind::Deb
            }
        }
        Some("appimage") => InstallKind::AppImage,
        _ => InstallKind::Unmanaged,
    }
}

/// The package tools this machine has, looked up where only root can write.
///
/// `$PATH` is not consulted: the command shown to a person runs as root, and a
/// path somebody else can prepend to is a command somebody else chose.
const TRUSTED: [&str; 4] = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];

pub(crate) fn tools_found() -> Vec<String> {
    let mut found = Vec::new();
    for tool in ["apt", "dpkg"] {
        for dir in TRUSTED {
            let path = std::path::Path::new(dir).join(tool);
            if path.is_file() {
                found.push(path.display().to_string());
                break;
            }
        }
    }
    found
}

/// What this build is, asked of the machine it is running on.
pub(crate) fn kind_here() -> InstallKind {
    let appimage = std::env::var("APPIMAGE").ok();
    let bundle = std::env::var("DEVPIT_BUNDLE_TYPE").ok();
    install_kind(appimage.as_deref(), bundle.as_deref(), &tools_found())
}

/// `update.check` — ask the feed whether there is a newer devpit.
///
/// Answers a state rather than a version: the window draws the state, and the
/// same states come back from the automatic check.
#[tauri::command]
#[specta::specta]
pub async fn update_check(app: tauri::AppHandle) -> Result<UpdateStatus, RpcError> {
    use tauri_plugin_updater::UpdaterExt;

    let kind = kind_here();
    let checking = next(&UpdateStatus::Idle, Event::Check)
        .ok_or_else(|| RpcError::internal("a check was refused before it started"))?;
    if kind == InstallKind::ExternallyManaged {
        return Ok(UpdateStatus::ExternallyManaged);
    }

    let heard = match app.updater() {
        Err(err) => Event::Failed {
            message: format!("no updater: {err}"),
        },
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => Event::Found {
                version: update.version.clone(),
                notes: update.body.clone().unwrap_or_default(),
                kind,
            },
            Ok(None) => Event::NothingNewer,
            Err(err) => Event::Failed {
                message: format!("could not check for an update: {err}"),
            },
        },
    };

    // The table decides, here as everywhere: a command that builds its own
    // states is a second opinion about what may follow what.
    next(&checking, heard).ok_or_else(|| RpcError::internal("the check ended nowhere"))
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
