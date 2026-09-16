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
        /// True when the answer came from `DEVPIT_UPDATE_FEED_FILE`.
        test_feed: bool,
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
                test_feed,
            },
        ) => Some(match kind {
            InstallKind::ExternallyManaged => S::ExternallyManaged,
            kind => S::Available {
                version,
                notes,
                kind,
                test_feed,
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

/// How long to wait before looking again.
const A_DAY: f64 = 24.0 * 60.0 * 60.0;
const AFTER_A_FAILURE: f64 = 60.0 * 60.0;
const AT_MOST: f64 = 6.0 * 60.0 * 60.0;

/// How often the task wakes to ask `due`.
const A_CYCLE: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// Whether to check now.
///
/// At startup, then once a day; after a failure an hour, doubling to six, so a
/// machine that is offline for a morning does not ask sixty times.
pub(crate) fn due(last: Option<f64>, now: f64, failures: u32, enabled: bool) -> bool {
    if !enabled {
        return false;
    }
    let Some(last) = last else {
        return true;
    };
    let wait = if failures == 0 {
        A_DAY
    } else {
        (AFTER_A_FAILURE * 2f64.powi(failures as i32 - 1)).min(AT_MOST)
    };
    now - last >= wait
}

/// The feed a test points the check at, instead of the network.
///
/// The same category of surface as a command that only exists for tests, and
/// accepted for the same reasons it was refused there: it only replaces where
/// the answer is *read* from, it is visible on screen as a test feed, and
/// nothing it offers can be installed.
pub(crate) fn fixture_feed() -> Option<std::path::PathBuf> {
    std::env::var_os("DEVPIT_UPDATE_FEED_FILE")
        .map(std::path::PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

/// What a fixture feed says, read as the static manifest it is.
fn feed_says(path: &std::path::Path, current: &str) -> Event {
    let Ok(text) = std::fs::read_to_string(path) else {
        return Event::Failed {
            message: format!("the test feed at {} could not be read", path.display()),
        };
    };
    let Ok(feed) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Event::Failed {
            message: "the test feed is not the manifest shape".to_owned(),
        };
    };
    let version = feed
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    if version.is_empty() || version == current {
        return Event::NothingNewer;
    }
    Event::Found {
        version,
        notes: feed
            .get("notes")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned(),
        kind: kind_here(),
        test_feed: true,
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

/// Looks for a newer devpit on its own, while the switch says to.
///
/// One line per transition on stderr and one event to the window: a person who
/// leaves the app open for a week should not have to ask, and a person who
/// turned the switch off should never be asked of the network at all.
///
/// The switch is read every cycle rather than once: turning it off is meant to
/// take effect without a restart.
pub(crate) fn watch(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last: Option<f64> = None;
        let mut failures: u32 = 0;
        let mut state = UpdateStatus::Idle;

        loop {
            // Unset means on: this is the switch a new install never touched.
            let enabled = devpit_core::Store::open_default()
                .and_then(|store| store.preference_flag(devpit_core::preference::AUTO_UPDATE))
                .ok()
                .flatten()
                .unwrap_or(true);

            if due(last, now(), failures, enabled) {
                last = Some(now());
                let heard = match update_check(app.clone()).await {
                    Ok(heard) => heard,
                    Err(err) => UpdateStatus::Failed {
                        message: err.message.clone(),
                        recoverable: true,
                    },
                };
                failures = match &heard {
                    UpdateStatus::Failed { .. } => failures.saturating_add(1),
                    _ => 0,
                };
                if heard != state {
                    state = heard.clone();
                    // The state, and nothing else: no url, no bytes, no
                    // version of anyone's machine.
                    eprintln!("devpit-update {}", named(&state));
                    let _ = tauri::Emitter::emit(&app, "update:status", &state);
                }
            }

            tokio::time::sleep(A_CYCLE).await;
        }
    });
}

/// The word for a state, for the one line it gets on stderr.
fn named(state: &UpdateStatus) -> &'static str {
    match state {
        UpdateStatus::Idle => "idle",
        UpdateStatus::Checking => "checking",
        UpdateStatus::Available { .. } => "available",
        UpdateStatus::Downloading { .. } => "downloading",
        UpdateStatus::Ready { .. } => "ready",
        UpdateStatus::Waiting { .. } => "waiting",
        UpdateStatus::Installing => "installing",
        UpdateStatus::ManualInstall { .. } => "manual-install",
        UpdateStatus::ExternallyManaged => "externally-managed",
        UpdateStatus::Failed { .. } => "failed",
    }
}

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_secs_f64())
        .unwrap_or_default()
}

/// `update.check` — ask the feed whether there is a newer devpit.
///
/// Answers a state rather than a version: the window draws the state, and the
/// same states come back from the automatic check.
#[tauri::command]
#[specta::specta]
pub async fn update_check(app: tauri::AppHandle) -> Result<UpdateStatus, RpcError> {
    let kind = kind_here();
    let checking = next(&UpdateStatus::Idle, Event::Check)
        .ok_or_else(|| RpcError::internal("a check was refused before it started"))?;
    if kind == InstallKind::ExternallyManaged {
        return Ok(UpdateStatus::ExternallyManaged);
    }

    let heard = if let Some(feed) = fixture_feed() {
        feed_says(&feed, env!("CARGO_PKG_VERSION"))
    } else {
        asked(&app, kind).await
    };

    // The table decides, here as everywhere: a command that builds its own
    // states is a second opinion about what may follow what.
    next(&checking, heard).ok_or_else(|| RpcError::internal("the check ended nowhere"))
}

/// What the feed answers, through the plugin.
async fn asked(app: &tauri::AppHandle, kind: InstallKind) -> Event {
    use tauri_plugin_updater::UpdaterExt;

    match app.updater() {
        Err(err) => Event::Failed {
            message: format!("no updater: {err}"),
        },
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => Event::Found {
                version: update.version.clone(),
                notes: update.body.clone().unwrap_or_default(),
                kind,
                test_feed: false,
            },
            Ok(None) => Event::NothingNewer,
            Err(err) => Event::Failed {
                message: format!("could not check for an update: {err}"),
            },
        },
    }
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
