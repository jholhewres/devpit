//! The shell a terminal window starts, and the startup file behind it.

use devpit_core::Store;
use devpit_rpc::{KnownAgent, PaneRunning, RpcError};

/// The shell a new window starts, with the startup file that makes it speak.
///
/// A shell tmux starts on its own says nothing about itself, so the OSC
/// scanner has nothing to read: no prompt boundary, no exit code, no way to
/// tell a command apart from its output. The startup file installs the hooks
/// that emit them.
///
/// A failure to write it is not a failure to open a terminal. The person gets
/// their shell, unwrapped and quiet, rather than no shell at all.
pub(crate) fn wrapped_shell() -> Result<devpit_tmux::Shell, RpcError> {
    use devpit_pty::shell;

    let program = shell::preferred(std::env::var("SHELL").ok().as_deref());
    let root = shell::root_for(&Store::root()?);
    // Marks only. `Identity` is emitted once, as the shell starts, and a
    // client attaches later — measured, it never arrives, and a feature that
    // cannot reach anyone is worse on than off.
    let features: &[shell::Feature] = if shell::install(&root).is_ok() {
        &[shell::Feature::Marks]
    } else {
        &[]
    };
    let launch = shell::launch(
        &program,
        &root,
        features,
        std::env::var("ZDOTDIR").ok().as_deref(),
    );
    Ok(devpit_tmux::Shell {
        program: launch.program,
        args: launch.args,
        env: launch.env,
    })
}

/// `session.running` — what each of a project's panes has in the foreground.
///
/// Asked of tmux and of the process table, which is asking the kernel.
/// Nothing is installed in the person's own configuration to make this work,
/// and nothing leaves the machine: the alternative is writing hooks into every
/// agent CLI's settings file, which reaches further and needs consent this
/// does not.
///
/// Two questions, because one of them is not enough. tmux says which window
/// owns which terminal and what executable is in front of it; the process
/// table says what that executable was *given*, which is where an agent's name
/// actually is. Every JavaScript agent — Claude Code, Codex, Gemini, OpenCode
/// — runs as `node`, so tmux alone reported `node` and the sidebar showed
/// nothing while a conversation was happening in front of it.
///
/// A project with no session yet is not an error — it is an empty list. The
/// sidebar polls this, and a refusal on every tick for a project nobody has
/// opened a terminal in would be noise.
///
/// Off the main thread, because it is asked on a timer. A synchronous Tauri
/// command runs on the thread that draws the window, and this one spawns tmux
/// and reads the process table — measured at about forty milliseconds, every
/// two seconds, on the thread that draws. `spawn_blocking` and not a plain
/// `async fn`: the body blocks, and blocking a runtime worker only moves the
/// stall somewhere less visible.
#[tauri::command]
#[specta::specta]
pub async fn session_running(project_id: String) -> Result<Vec<PaneRunning>, RpcError> {
    tauri::async_runtime::spawn_blocking(move || running_in(&project_id))
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?
}

fn running_in(project_id: &str) -> Result<Vec<PaneRunning>, RpcError> {
    let Ok(server) = crate::sessions::tmux_server() else {
        return Ok(Vec::new());
    };
    let session = devpit_tmux::Server::session_name(project_id);
    let Ok(panes) = server.running(&session) else {
        return Ok(Vec::new());
    };

    // One `ps` for every pane at once. A spawn per row, on a two-second
    // timer, would cost more than the answer is worth.
    let ttys: Vec<String> = panes.iter().map(|one| one.tty.clone()).collect();
    let fronts = devpit_pty::looking(&ttys);

    Ok(panes.into_iter().map(|one| named(&fronts, one)).collect())
}

/// What one pane is running, once both sources have been asked.
fn named(fronts: &[devpit_pty::Front], pane: devpit_tmux::Running) -> PaneRunning {
    // tmux's answer is the fallback, not the answer: it is right for a native
    // binary and wrong for every interpreted one, and it is all there is when
    // `ps` could not be read.
    let command = devpit_pty::front_on(fronts, &pane.tty)
        .and_then(|front| devpit_pty::agents::program_of(&front.argv))
        .unwrap_or(pane.command);

    let agent = devpit_pty::front_on(fronts, &pane.tty)
        .and_then(|front| devpit_pty::agents::recognise(&front.argv));

    PaneRunning {
        pane_id: pane.leaf_id,
        busy: !devpit_pty::agents::idle_shell(&command),
        label: agent
            .map(|one| one.label.to_owned())
            .unwrap_or_else(|| command.clone()),
        agent: agent.map(|one| one.id.to_owned()),
        command,
    }
}

/// `agents.known` — the agent CLIs this build can start, and which are here.
///
/// The list the menu draws. It is the same list that recognises a running
/// agent, on purpose: two lists drift, and the drift shows up as starting
/// Gemini from our own menu and then being told the pane is running `node`.
#[tauri::command]
#[specta::specta]
pub async fn agents_known() -> Result<Vec<KnownAgent>, RpcError> {
    // `spawn_blocking`, because the first call can be the one that waits for
    // the shell probe. Warmed at startup, so in practice it is already there
    // — but "in practice" is not where a blocked runtime worker comes from.
    tauri::async_runtime::spawn_blocking(|| {
        devpit_pty::agents::KNOWN
            .iter()
            .map(|one| KnownAgent {
                id: one.id.to_owned(),
                label: one.label.to_owned(),
                launch: one.launch.to_owned(),
                installed: installed().contains(one.id),
            })
            .collect()
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))
}

/// Which agents this machine can actually start.
///
/// Asked of the person's own login shell, once, and remembered.
///
/// Walking `PATH` was the first cut and it was wrong on the machine it was
/// written on: `claude` there is a **shell function** defined in `.zshrc`,
/// with no file anywhere on `PATH`. A menu that walks the filesystem greys out
/// the one agent its owner actually uses. What starts an agent is the shell,
/// so the shell is what gets asked.
///
/// One invocation for the whole list, because an interactive shell is not
/// cheap: measured at 1.6 seconds here, which is fine once and absurd twelve
/// times. Warmed at startup by [`warm_installed`], so no menu waits for it.
fn installed() -> &'static std::collections::HashSet<String> {
    static FOUND: std::sync::OnceLock<std::collections::HashSet<String>> =
        std::sync::OnceLock::new();
    FOUND.get_or_init(ask_the_shell)
}

/// Starts the probe now, so the first menu finds the answer already there.
pub(crate) fn warm_installed() {
    std::thread::spawn(|| {
        let _ = installed();
    });
}

#[cfg(unix)]
fn ask_the_shell() -> std::collections::HashSet<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    // Interactive, because that is the only mode that reads the file where a
    // function or an alias would be defined.
    let script = devpit_pty::agents::KNOWN
        .iter()
        .map(|one| {
            let program = one.launch.split_whitespace().next().unwrap_or(one.launch);
            format!("command -v {program} >/dev/null 2>&1 && echo {}", one.id)
        })
        .collect::<Vec<_>>()
        .join("; ");

    let Ok(output) = std::process::Command::new(&shell)
        .args(["-ic", &script])
        // Nothing to read. An interactive shell that inherits a terminal can
        // sit waiting on it forever, and this runs at startup — a hang here
        // would be an app that never finishes opening.
        .stdin(std::process::Stdio::null())
        .output()
    else {
        return std::collections::HashSet::new();
    };
    // The exit status is the last test's, so it says nothing about the rest.
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(not(unix))]
fn ask_the_shell() -> std::collections::HashSet<String> {
    // Windows has no `-ic`, and the daemon is what fills this seam there.
    // Until then every agent is offered and the shell says if it is missing.
    devpit_pty::agents::KNOWN
        .iter()
        .map(|one| one.id.to_owned())
        .collect()
}

/// `session.launch_agent` — types an agent's launch line into a pane.
///
/// Typed into the pane rather than spawned beside it, and that is the whole
/// design: the person sees the command they would have typed, in the shell
/// they are in, with their own `PATH` and their own configuration. A process
/// started behind the terminal would be an agent the terminal does not own,
/// and closing the tab would leave it running with nothing to reach it.
///
/// The line is not sent blind, and it is not sent early.
///
/// A pane with something already in front of it would take the text as input
/// to *that* — a prompt typed into an agent that is already open — so a busy
/// pane is refused with the reason.
///
/// And a shell that has not reached its prompt yet drops what it is sent.
/// Measured on the machine this was written on: a fresh tmux window took 1.3
/// seconds to print its first prompt, and a line sent at half a second simply
/// vanished. So this waits, which is why it is async.
#[tauri::command]
#[specta::specta]
pub async fn session_launch_agent(
    state: tauri::State<'_, crate::sessions::SessionState>,
    project_id: String,
    pane_id: String,
    agent_id: String,
) -> Result<String, RpcError> {
    let agent = devpit_pty::agents::known(&agent_id).ok_or_else(|| {
        RpcError::new(
            devpit_rpc::ErrorCode::NotFound,
            format!("{agent_id} is not an agent this build knows"),
        )
    })?;

    // The pane has to be this project's. Reached from a menu, the id comes
    // from the screen, and the screen is not the authority on what is open.
    let layout = crate::sessions::holding(&project_id, &pane_id)?;
    let session = devpit_tmux::Server::session_name(&layout.project_id);
    let target = devpit_tmux::Server::target(&session, &pane_id);

    // Not under the project lock: this waits seconds, and holding the lock
    // that long would stall every other thing the project wants to do.
    let waiting = session.clone();
    let waited = pane_id.clone();
    let ready = tauri::async_runtime::spawn_blocking(move || settled(&waiting, &waited))
        .await
        .map_err(|err| RpcError::internal(err.to_string()))?;

    if let Ready::Busy(command) = ready {
        return Err(RpcError::new(
            devpit_rpc::ErrorCode::Conflict,
            format!("this terminal is running {command}"),
        ));
    }

    let line = launch_line(agent);
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    crate::sessions::tmux_server()?
        .send_keys(&target, &line)
        .map_err(crate::sessions::tmux_err)?;
    Ok(line)
}

/// The command this agent is started with, hooks and all.
///
/// The hooks are how "an agent is open" becomes "the agent is waiting for
/// you": the process table can say which program is in front of a terminal,
/// and only the agent itself can say what it is doing.
///
/// On the command line rather than in the person's own settings file. Orca
/// installs its hooks into `~/.claude/settings.json`, which reaches every
/// agent they ever start — including the ones they start outside this app —
/// and that is a change to files in somebody's home directory that a menu
/// item does not get to make on its own. The flag reaches exactly the agents
/// this app started, which is the set it has any business watching.
///
/// Without the settings file the line is just the agent's name, which is what
/// it always was.
fn launch_line(agent: &devpit_pty::agents::Known) -> String {
    let Some(flag) = agent.settings_flag else {
        return agent.launch.to_owned();
    };
    let Some(settings) = hook_settings() else {
        return agent.launch.to_owned();
    };
    format!("{} {}", agent.launch, flag.replace("{}", &settings))
}

/// Where the hook settings live, written if they are not there yet.
///
/// The same file the board's headless turns use, and written by the same
/// rule: only when it differs, so starting an agent does not touch the disk
/// for nothing.
fn hook_settings() -> Option<String> {
    let root = Store::root().ok()?;
    let endpoint = devpit_agentcli::endpoint_file(&root);
    let path = root.join("hooks.json");
    let wanted = devpit_agentcli::settings_json(&endpoint);
    if std::fs::read_to_string(&path).ok().as_deref() != Some(wanted.as_str()) {
        std::fs::create_dir_all(&root).ok()?;
        std::fs::write(&path, &wanted).ok()?;
    }
    // Quoted, because a home directory with a space in it would otherwise
    // become two arguments to the shell this is typed into.
    Some(format!("'{}'", path.display()))
}

/// Whether a pane is ready to be typed into.
enum Ready {
    /// A shell, at its prompt, waiting on the keyboard.
    Prompt,
    /// Something else is in front of it, named.
    Busy(String),
    /// It never settled. Sending anyway beats refusing: the worst case is a
    /// line the shell drops, and refusing is a menu item that does nothing.
    Unknown,
}

/// How long to wait for a shell to reach its prompt.
///
/// Generous on purpose. A heavy `.zshrc` is the person's own, and taking a
/// second and a half to start is not an error to report at them — it is a
/// second and a half to wait through.
const AT_MOST: std::time::Duration = std::time::Duration::from_secs(8);

/// How often to look, and how long a rest has to hold before it counts.
const BETWEEN_LOOKS: std::time::Duration = std::time::Duration::from_millis(90);

fn settled(session: &str, pane_id: &str) -> Ready {
    let Ok(server) = crate::sessions::tmux_server() else {
        return Ready::Unknown;
    };
    let deadline = std::time::Instant::now() + AT_MOST;
    let mut settling = devpit_pty::Settling::new();

    while std::time::Instant::now() < deadline {
        let Ok(panes) = server.running(session) else {
            return Ready::Unknown;
        };
        let Some(pane) = panes.into_iter().find(|one| one.leaf_id == pane_id) else {
            return Ready::Unknown;
        };
        let fronts = devpit_pty::looking(std::slice::from_ref(&pane.tty));
        let Some(front) = devpit_pty::front_on(&fronts, &pane.tty) else {
            return Ready::Unknown;
        };

        let command = devpit_pty::agents::program_of(&front.argv).unwrap_or(pane.command);
        if !devpit_pty::agents::idle_shell(&command) {
            return Ready::Busy(command);
        }
        if settling.looked(devpit_pty::at_a_prompt(front)) {
            return Ready::Prompt;
        }
        std::thread::sleep(BETWEEN_LOOKS);
    }
    Ready::Unknown
}
