//! The rules an update follows, and the check that uses them.
//!
//! Three of them are pure, because they are the ones worth arguing about: what
//! may happen next (`next`), when to look again (`due`), and what this copy of
//! devpit even is (`install_kind`). Everything else here is plumbing around
//! the updater plugin.

use tauri::Manager;

use devpit_rpc::{InstallKind, RpcError, UpdateBlocking, UpdateStatus, UpdateWork};

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
    Download,
    Progress(u8),
    /// Work is still running, so the install waits for it.
    Blocked {
        runs: u32,
        turns: u32,
        since: f64,
    },
    /// Downloaded, and verified by the plugin before the bytes came back.
    Downloaded {
        version: String,
    },
    /// A package this app will not install: the person runs the command.
    Manual {
        command: String,
        path: String,
    },
    /// Not now.
    Cancel,
    /// Put it in.
    Install,
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

        (S::Available { .. }, E::Download) => Some(S::Downloading { percent: 0 }),
        (S::Downloading { .. }, E::Progress(percent)) => Some(S::Downloading {
            percent: percent.min(100),
        }),
        (S::Downloading { .. }, E::Downloaded { version }) => Some(S::Ready { version }),
        (S::Downloading { .. }, E::Manual { command, path }) => {
            Some(S::ManualInstall { command, path })
        }
        (S::Ready { .. }, E::Blocked { runs, turns, since }) => {
            Some(S::Waiting { runs, turns, since })
        }
        (S::Waiting { .. }, E::Cancel) | (S::Ready { .. }, E::Cancel) => Some(S::Idle),
        (S::Ready { .. } | S::Waiting { .. }, E::Install) => Some(S::Installing),
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
/// How often a chosen wait looks at whether the work has ended.
const WHILE_WAITING: std::time::Duration = std::time::Duration::from_secs(5);

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
    let running_as_appimage = appimage.is_some_and(|path| !path.is_empty());
    match bundle_type {
        // Both, not either: `$APPIMAGE` is inherited by every process the app
        // starts, tmux panes included, so a .deb build launched from one of
        // them would otherwise be sent down the path that replaces a file.
        Some("appimage") if running_as_appimage => InstallKind::AppImage,
        Some("deb") | Some("rpm") => {
            if tools.is_empty() {
                InstallKind::ExternallyManaged
            } else {
                InstallKind::Deb
            }
        }
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

/// What this build is: the bundle type the bundler stamped into the binary,
/// and where it is running from.
pub(crate) fn kind_here() -> InstallKind {
    use tauri::utils::config::BundleType;
    let appimage = std::env::var("APPIMAGE").ok();
    let stamped = tauri::utils::platform::bundle_type().map(|stamp| match stamp {
        BundleType::AppImage => "appimage",
        BundleType::Deb => "deb",
        BundleType::Rpm => "rpm",
        _ => "other",
    });
    install_kind(appimage.as_deref(), stamped, &tools_found())
}

/// What the app knows about the update in flight.
///
/// The `Update` the plugin handed back is kept because downloading needs it,
/// and the bytes are kept because installing needs them: a second check
/// between the two would be a second answer about which version this is.
#[derive(Default)]
pub struct Updating {
    pub(crate) status: std::sync::Mutex<Option<UpdateStatus>>,
    /// Raised by the window when it has saved what it had.
    pub(crate) window_ready: tokio::sync::Notify,
    pub(crate) found: std::sync::Mutex<Option<tauri_plugin_updater::Update>>,
    pub(crate) downloaded: std::sync::Mutex<Option<(String, Vec<u8>)>>,
    /// A package written to the cache, and what it hashed to when it was
    /// verified. Checked again before its command is shown.
    pub(crate) package: std::sync::Mutex<Option<(std::path::PathBuf, String)>>,
}

impl Updating {
    pub(crate) fn state(&self) -> UpdateStatus {
        self.status
            .lock()
            .ok()
            .and_then(|held| held.clone())
            .unwrap_or(UpdateStatus::Idle)
    }

    fn moved_to(&self, app: &tauri::AppHandle, status: UpdateStatus) {
        if let Ok(mut held) = self.status.lock() {
            *held = Some(status.clone());
        }
        eprintln!("devpit-update {}", named(&status));
        let _ = tauri::Emitter::emit(app, "update:status", &status);
    }
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

        loop {
            let current = app
                .try_state::<Updating>()
                .map(|updating| updating.state())
                .unwrap_or(UpdateStatus::Idle);

            // "When it is done" is a promise to go in once the work has ended,
            // and nothing else would keep it: new work is refused meanwhile,
            // so without this the app would wait for ever.
            if matches!(current, UpdateStatus::Waiting { .. }) {
                let idle = app
                    .try_state::<crate::chat::Talking>()
                    .and_then(|talking| update_running(talking).ok())
                    .is_some_and(|work| blockers(&work) == 0);
                if idle {
                    if let Some(updating) = app.try_state::<Updating>() {
                        let _ = update_install(app.clone(), updating).await;
                    }
                }
                tokio::time::sleep(WHILE_WAITING).await;
                continue;
            }

            // Unset means on: this is the switch a new install never touched.
            let enabled = devpit_core::Store::open_default()
                .and_then(|store| store.preference_flag(devpit_core::preference::AUTO_UPDATE))
                .ok()
                .flatten()
                .unwrap_or(true);

            // Only when a check may start from here: one in the middle of a
            // download is refused, and counting that as a failure would back
            // the next check off for nothing.
            if next(&current, Event::Check).is_some() && due(last, now(), failures, enabled) {
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
                if heard != current {
                    // The state, and nothing else: no url, no bytes, no
                    // version of anyone's machine.
                    eprintln!("devpit-update {}", named(&heard));
                    let _ = tauri::Emitter::emit(&app, "update:status", &heard);
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
    // From where things are, not from Idle: a check in the middle of a
    // download replaced the update being downloaded and dropped its bytes.
    let now = app
        .try_state::<Updating>()
        .map(|updating| updating.state())
        .unwrap_or(UpdateStatus::Idle);
    let checking = next(&now, Event::Check).ok_or_else(|| {
        RpcError::new(
            devpit_rpc::ErrorCode::Conflict,
            "an update is already on its way".to_owned(),
        )
    })?;
    if kind == InstallKind::ExternallyManaged {
        return Ok(UpdateStatus::ExternallyManaged);
    }

    let (heard, found) = if let Some(feed) = fixture_feed() {
        (feed_says(&feed, env!("CARGO_PKG_VERSION")), None)
    } else {
        asked(&app, kind).await
    };
    if let Some(updating) = app.try_state::<Updating>() {
        if let Ok(mut held) = updating.found.lock() {
            *held = found;
        }
    }

    // The table decides, here as everywhere: a command that builds its own
    // states is a second opinion about what may follow what.
    let ended =
        next(&checking, heard).ok_or_else(|| RpcError::internal("the check ended nowhere"))?;
    if let Some(updating) = app.try_state::<Updating>() {
        updating.moved_to(&app, ended.clone());
    }
    Ok(ended)
}

/// What the feed answers, through the plugin, and the update it answered with.
///
/// The update comes back too: downloading needs the very object the check
/// produced, and asking again would be a second answer about which version
/// this is.
async fn asked(
    app: &tauri::AppHandle,
    kind: InstallKind,
) -> (Event, Option<tauri_plugin_updater::Update>) {
    use tauri_plugin_updater::UpdaterExt;

    match app.updater() {
        Err(err) => (
            Event::Failed {
                message: format!("no updater: {err}"),
            },
            None,
        ),
        Ok(updater) => match updater.check().await {
            Ok(Some(update)) => (
                Event::Found {
                    version: update.version.clone(),
                    notes: update.body.clone().unwrap_or_default(),
                    kind,
                    test_feed: false,
                },
                Some(update),
            ),
            Ok(None) => (Event::NothingNewer, None),
            Err(err) => (
                Event::Failed {
                    message: format!("could not check for an update: {err}"),
                },
                None,
            ),
        },
    }
}

/// What a person chose to do about the work in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Choice {
    /// Wait for it to finish, then install.
    WhenItIsDone,
    /// Stop it and install now.
    StopIt,
    /// Not now.
    Later,
}

/// What the app does about an update, given the choice and what is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Plan {
    /// Nothing is running: install.
    InstallNow,
    /// Hold the update until the work ends.
    WaitForIdle,
    /// Stop the work, then install.
    StopThenInstall,
    Later,
}

/// What an update would be waiting for.
///
/// Pure over the two lists so the rule can be read without a store: a run with
/// no card title still blocks, and an empty pair is the only thing that lets an
/// install go straight through.
pub(crate) fn blockers(work: &UpdateWork) -> usize {
    work.runs.len() + work.turns.len()
}

/// The plan, from the choice and what is in the way.
pub(crate) fn install_plan(choice: Choice, work: &UpdateWork) -> Plan {
    if choice == Choice::Later {
        return Plan::Later;
    }
    if blockers(work) == 0 {
        return Plan::InstallNow;
    }
    match choice {
        Choice::WhenItIsDone => Plan::WaitForIdle,
        Choice::StopIt => Plan::StopThenInstall,
        Choice::Later => Plan::Later,
    }
}

/// Why new work is refused while an update goes in.
///
/// Only once the person chose to wait for the work to end, or the install has
/// begun — not merely because an update is downloaded. Refusing from `Ready`
/// meant a closed card left every run and chat refused until a restart.
pub(crate) fn starting_refused(state: &UpdateStatus) -> Option<&'static str> {
    match state {
        UpdateStatus::Waiting { .. } => {
            Some("an update is waiting for the work to finish, so nothing new is started")
        }
        UpdateStatus::Installing => Some("an update is installing"),
        _ => None,
    }
}

/// What a person would recognise the work in flight by.
///
/// Titles rather than ids: "two runs" is a number, and the question on screen
/// is whether *this* is worth interrupting.
#[tauri::command]
#[specta::specta]
pub fn update_running(
    talking: tauri::State<'_, crate::chat::Talking>,
) -> Result<UpdateWork, RpcError> {
    let store = devpit_core::Store::open_default()?;
    let runs = store
        .running_runs()?
        .into_iter()
        .map(|(id, title)| UpdateBlocking { id, title })
        .collect();

    let turns = talking
        .running
        .lock()
        .map(|held| held.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default()
        .into_iter()
        .map(|conversation_id| {
            // A conversation is named by the card it is about; one that is
            // about no card is named by itself rather than by nothing.
            let title = store
                .chat_card(&conversation_id)
                .ok()
                .flatten()
                .and_then(|card_id| store.card(&card_id).ok().flatten())
                .map(|card| card.title)
                .unwrap_or_else(|| conversation_id.clone());
            UpdateBlocking {
                id: conversation_id,
                title,
            }
        })
        .collect();

    Ok(UpdateWork { runs, turns })
}

/// What putting an update in takes, in order.
///
/// A list rather than a function body so it can be read and asserted about:
/// the one thing that must never be in it is anything that touches tmux. The
/// terminals outlive this process by design — that is the whole reason an
/// update can be installed while they are running — and killing the server
/// here would take every session on the machine with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuitStep {
    /// Let the window save what it has, with a short deadline.
    AskTheWindow,
    /// Hand the verified bytes to the installer. Past this, there is no back.
    Install,
    /// Start the new one.
    Restart,
    /// Show the command instead: a package devpit never installs itself.
    ShowTheCommand,
}

pub(crate) fn quit_steps(kind: InstallKind) -> Vec<QuitStep> {
    match kind {
        InstallKind::AppImage => vec![QuitStep::AskTheWindow, QuitStep::Install, QuitStep::Restart],
        InstallKind::Deb => vec![QuitStep::ShowTheCommand],
        // Nothing to do to a build nobody installs from here.
        InstallKind::Unmanaged | InstallKind::ExternallyManaged => Vec::new(),
    }
}

/// How long the process gets to go away after the installer has been called.
const BEFORE_FORCING_THE_EXIT: std::time::Duration = std::time::Duration::from_secs(20);

/// Leaves, whatever else is holding on.
///
/// The installer is waiting for this process to end so it can replace the
/// file; a teardown stuck on a socket or a watcher would leave the update
/// half-applied and the person on the old version with no way to be told why.
fn force_the_exit_eventually() {
    std::thread::spawn(|| {
        std::thread::sleep(BEFORE_FORCING_THE_EXIT);
        eprintln!(
            "devpit-update forcing the exit: still here {}s after the installer was called",
            BEFORE_FORCING_THE_EXIT.as_secs()
        );
        std::process::exit(0);
    });
}

/// How long the window gets to save what it has before the update goes on.
const THE_WINDOW_GETS: std::time::Duration = std::time::Duration::from_millis(2500);

/// Waits for the window to say it is ready, or gives up.
///
/// Fail open, deliberately: a window that never answers — busy, wedged, or
/// already gone — must not be able to hold an update forever. Answers whether
/// it was the window that released it, which is what the tests read.
pub(crate) async fn window_saved(ready: &tokio::sync::Notify, within: std::time::Duration) -> bool {
    tokio::time::timeout(within, ready.notified()).await.is_ok()
}

/// `update.restart_ready` — the window has saved what it had.
#[tauri::command]
#[specta::specta]
pub fn update_restart_ready(updating: tauri::State<'_, Updating>) {
    updating.window_ready.notify_one();
}

/// `update.package` — the command for the package that was downloaded.
///
/// Checked again here rather than trusted from the download: a person may come
/// back to this card hours later, and the file has been sitting in a
/// world-readable cache the whole time. A file that is no longer what was
/// verified gets its own sentence and no command at all.
#[tauri::command]
#[specta::specta]
pub fn update_package(updating: tauri::State<'_, Updating>) -> Result<String, RpcError> {
    let (path, digest) = updating
        .package
        .lock()
        .ok()
        .and_then(|held| held.clone())
        .ok_or_else(|| RpcError::internal("no package has been downloaded"))?;

    let folder = crate::update_deb::cache_dir();
    crate::update_deb::still_ours(&path, &folder, &digest)
        .map_err(|why| RpcError::new(devpit_rpc::ErrorCode::Conflict, why.said().to_owned()))?;

    crate::update_deb::install_command(&path, &crate::update_deb::TRUSTED)
        .map_err(|why| RpcError::new(devpit_rpc::ErrorCode::Conflict, why))
}

/// `update.install` — put it in and come back.
///
/// Only an AppImage is installed from here: a `.deb` is shown as a command for
/// the person to run (US-017), because installing it means asking for root and
/// devpit never does that on anyone's behalf.
#[tauri::command]
#[specta::specta]
pub async fn update_install(
    app: tauri::AppHandle,
    updating: tauri::State<'_, Updating>,
) -> Result<UpdateStatus, RpcError> {
    let state = updating.state();
    if !matches!(
        state,
        UpdateStatus::Ready { .. } | UpdateStatus::Waiting { .. }
    ) {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Conflict,
            "there is nothing ready to install".to_owned(),
        ));
    }

    let kind = kind_here();
    let steps = quit_steps(kind);
    if !steps.contains(&QuitStep::Install) {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Conflict,
            "this build is not one devpit installs over".to_owned(),
        ));
    }

    // The update first: taking the bytes and then finding nothing to install
    // them with would throw away a download that verified.
    let found = updating
        .found
        .lock()
        .ok()
        .and_then(|mut held| held.take())
        .ok_or_else(|| RpcError::internal("the update to install is no longer known"))?;
    let Some((version, bytes)) = updating
        .downloaded
        .lock()
        .ok()
        .and_then(|mut held| held.take())
    else {
        // Before the commit: the offer is still good, and asking again is all
        // it takes.
        if let Ok(mut held) = updating.found.lock() {
            *held = Some(found);
        }
        return Err(RpcError::internal(
            "there are no verified bytes to install — download it again",
        ));
    };

    let going = next(&state, Event::Install)
        .ok_or_else(|| RpcError::internal("an install the rules allow but the table does not"))?;
    updating.moved_to(&app, going);

    // The window gets a moment to put down what it is holding. Whatever it
    // says, the update goes on: this is the last point where waiting is
    // possible at all, and a window that never answers must not own it.
    let _ = tauri::Emitter::emit(&app, "update:before-restart", ());
    let saved = window_saved(&updating.window_ready, THE_WINDOW_GETS).await;
    if !saved {
        eprintln!("devpit-update the window did not answer in time; going on");
    }

    if let Err(err) = found.install(bytes) {
        eprintln!("devpit-update the installer refused: {err}");
        let failed = UpdateStatus::Failed {
            message: format!("the installer refused: {err}"),
            recoverable: false,
        };
        updating.moved_to(&app, failed.clone());
        return Ok(failed);
    }

    // Committed: the new version is in place, and this process has to end for
    // it to run. If the restart hangs, the exit does not.
    force_the_exit_eventually();
    eprintln!("devpit-update installed {version}; restarting");
    app.restart();
}

/// `update.choose` — what to do about the work in flight.
///
/// Answers the state the choice leaves behind. Installing itself is not here:
/// this is the moment a person decides, and from `Ready` onward nothing new
/// starts whatever they decide.
#[tauri::command]
#[specta::specta]
pub fn update_choose(
    app: tauri::AppHandle,
    updating: tauri::State<'_, Updating>,
    talking: tauri::State<'_, crate::chat::Talking>,
    choice: String,
) -> Result<UpdateStatus, RpcError> {
    let chose = match choice.as_str() {
        "whenItIsDone" => Choice::WhenItIsDone,
        "stopIt" => Choice::StopIt,
        "later" => Choice::Later,
        other => {
            return Err(RpcError::new(
                devpit_rpc::ErrorCode::Invalid,
                format!("no such choice: {other}"),
            ))
        }
    };

    let work = update_running(talking)?;
    let state = updating.state();
    let moved = match install_plan(chose, &work) {
        Plan::Later => next(&state, Event::Cancel),
        Plan::WaitForIdle => next(
            &state,
            Event::Blocked {
                runs: work.runs.len() as u32,
                turns: work.turns.len() as u32,
                since: now(),
            },
        ),
        // Both mean "go", and going is US-016a's; the state stays where it is.
        Plan::InstallNow | Plan::StopThenInstall => None,
    };

    let ended = moved.unwrap_or(state);
    updating.moved_to(&app, ended.clone());
    Ok(ended)
}

/// Why a download would be refused, if it would.
///
/// Pure, because the answer is a rule rather than a step: a build nobody
/// installs from here, an offer from a test feed, and any state where a
/// download makes no sense are three different sentences, and the card shows
/// whichever one applies.
pub(crate) fn may_download(state: &UpdateStatus) -> Result<(), String> {
    match state {
        UpdateStatus::Available {
            kind, test_feed, ..
        } => {
            if *test_feed {
                return Err(
                    "this offer came from a test feed, so there is nothing to install".to_owned(),
                );
            }
            match kind {
                InstallKind::AppImage | InstallKind::Deb => Ok(()),
                InstallKind::Unmanaged => Err(
                    "this build was not installed from a devpit release; the release page has the files"
                        .to_owned(),
                ),
                InstallKind::ExternallyManaged => {
                    Err("this copy is looked after by your system, so update it there".to_owned())
                }
            }
        }
        UpdateStatus::Downloading { .. } => Err("this update is already downloading".to_owned()),
        UpdateStatus::Installing => Err("this update is already installing".to_owned()),
        _ => Err("there is nothing to download from here".to_owned()),
    }
}

/// `update.download` — fetch the update, with the plugin verifying it.
///
/// Refused from any state where a download makes no sense, and refused
/// outright for a build nobody installs from here: `make dev`, a `cargo run`,
/// or a package the machine looks after. The signature is checked by the
/// plugin before the bytes come back — see `updater.rs:740` — so what lands
/// here has already been proved to come from the key this app carries.
#[tauri::command]
#[specta::specta]
pub async fn update_download(
    app: tauri::AppHandle,
    updating: tauri::State<'_, Updating>,
) -> Result<UpdateStatus, RpcError> {
    let state = updating.state();
    may_download(&state).map_err(|why| RpcError::new(devpit_rpc::ErrorCode::Conflict, why))?;
    let going = next(&state, Event::Download)
        .ok_or_else(|| RpcError::internal("a download the rules allow but the table does not"))?;

    let found = updating
        .found
        .lock()
        .ok()
        .and_then(|mut held| held.take())
        .ok_or_else(|| RpcError::internal("the check did not leave an update to download"))?;

    updating.moved_to(&app, going);
    let version = found.version.clone();
    let told = app.clone();

    let mut seen = 0usize;
    let bytes = found
        .download(
            |chunk, total| {
                seen += chunk;
                if let Some(total) = total {
                    let percent = ((seen as f64 / total as f64) * 100.0).min(100.0) as u8;
                    if let Some(updating) = told.try_state::<Updating>() {
                        let now = updating.state();
                        if let Some(moved) = next(&now, Event::Progress(percent)) {
                            updating.moved_to(&told, moved);
                        }
                    }
                }
            },
            || {},
        )
        .await
        .map_err(|err| {
            let message = format!("the download failed: {err}");
            if let Some(updating) = app.try_state::<Updating>() {
                let now = updating.state();
                if let Some(moved) = next(
                    &now,
                    Event::Failed {
                        message: message.clone(),
                    },
                ) {
                    updating.moved_to(&app, moved);
                }
            }
            RpcError::internal(message)
        })?;

    // A package is not installed from here: it is written where the package
    // manager can read it, and the person is given the command.
    if kind_here() == InstallKind::Deb {
        let (path, digest) = crate::update_deb::keep(&bytes, &version)
            .map_err(|why| RpcError::internal(format!("the package could not be kept: {why}")))?;
        if let Ok(mut held) = updating.package.lock() {
            *held = Some((path.clone(), digest));
        }
        let command = crate::update_deb::install_command(&path, &crate::update_deb::TRUSTED)
            .unwrap_or_else(|why| why);
        let now = updating.state();
        let manual = next(
            &now,
            Event::Manual {
                command,
                path: path.display().to_string(),
            },
        )
        .ok_or_else(|| RpcError::internal("the package ended nowhere"))?;
        updating.moved_to(&app, manual.clone());
        return Ok(manual);
    }

    keep_for_install(
        &updating.found,
        &updating.downloaded,
        found,
        version.clone(),
        bytes,
    );
    let now = updating.state();
    let ready = next(&now, Event::Downloaded { version })
        .ok_or_else(|| RpcError::internal("the download ended nowhere"))?;
    updating.moved_to(&app, ready.clone());
    Ok(ready)
}

/// Keeps what an install needs, together: the update and its verified bytes.
///
/// `install` takes both. The download used to take the update out to fetch
/// its bytes and keep only the bytes, so every AppImage update reached
/// `update_install` with nothing to install it with — found by the rehearsal,
/// which is the one test that goes all the way to the install.
pub(crate) fn keep_for_install<U>(
    found: &std::sync::Mutex<Option<U>>,
    downloaded: &std::sync::Mutex<Option<(String, Vec<u8>)>>,
    update: U,
    version: String,
    bytes: Vec<u8>,
) {
    if let Ok(mut held) = found.lock() {
        *held = Some(update);
    }
    if let Ok(mut held) = downloaded.lock() {
        *held = Some((version, bytes));
    }
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
