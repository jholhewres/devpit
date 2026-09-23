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
    let mut env = launch.env;
    // `devpit-agent` on the terminal's PATH, so any agent with a shell — not
    // only the two that take an MCP flag — can reach the board. Named rather
    // than set: tmux ignores `-e PATH=`, and the startup file puts it on PATH
    // after the person's own config.
    if let Some(bin) = crate::agent_reach::exe()
        .and_then(|exe| crate::agent_reach::cli_shim(&Store::root().ok()?, &exe))
    {
        env.push(("DEVPIT_BIN".to_owned(), bin.display().to_string()));
    }
    env.extend(typed_agents());
    Ok(devpit_tmux::Shell {
        program: launch.program,
        args: launch.args,
        env,
    })
}

/// What the shell's startup file needs to start `claude` and `codex` typed
/// by hand the way devpit starts them: the hook settings, Claude's MCP file,
/// and the binary Codex is pointed at. Nothing, with the switch off.
fn typed_agents() -> Vec<(String, String)> {
    if !integrated() {
        return Vec::new();
    }
    let Ok(root) = Store::root() else {
        return Vec::new();
    };
    let mut env = Vec::new();
    if hook_settings().is_some() {
        env.push((
            "DEVPIT_CLAUDE_SETTINGS".to_owned(),
            root.join("hooks.json").display().to_string(),
        ));
    }
    if let Some(exe) = crate::agent_reach::exe() {
        let config = root.join("mcp.json");
        if crate::agent_reach::mcp_flags("claude", &exe, &config).is_some() {
            env.push(("DEVPIT_CLAUDE_MCP".to_owned(), config.display().to_string()));
        }
        if crate::agent_reach::mcp_flags("codex", &exe, &config).is_some() {
            env.push(("DEVPIT_CODEX_EXE".to_owned(), exe.display().to_string()));
        }
    }
    env
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
pub async fn session_running(
    app: tauri::AppHandle,
    project_id: String,
) -> Result<Vec<PaneRunning>, RpcError> {
    tauri::async_runtime::spawn_blocking(move || {
        // Read before tmux and `ps` are asked: a hook heard after this is
        // newer than anything they say.
        let seq = crate::card_activity::next_seq();
        let panes = running_in(&project_id)?;
        crate::card_reconcile::reconcile(&app, &project_id, &panes, seq);
        Ok(panes)
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))?
}

pub(crate) fn running_in(project_id: &str) -> Result<Vec<PaneRunning>, RpcError> {
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

    // Only asked when a pane says devpit started a profile in it, so the
    // ordinary session pays nothing for a feature it is not using.
    let names = if panes.iter().any(|one| !one.profile.is_empty()) {
        profile_names()
    } else {
        std::collections::HashMap::new()
    };

    Ok(panes
        .into_iter()
        .map(|one| named(&fronts, &names, one))
        .collect())
}

/// Every declared profile's name, by id.
fn profile_names() -> std::collections::HashMap<String, String> {
    crate::projects::store()
        .map(|store| crate::agent_profiles::names(&store))
        .unwrap_or_default()
}

/// What one pane is running, once every source has been asked.
fn named(
    fronts: &[devpit_pty::Front],
    names: &std::collections::HashMap<String, String>,
    pane: devpit_tmux::Running,
) -> PaneRunning {
    // tmux's answer is the fallback, not the answer: it is right for a native
    // binary and wrong for every interpreted one, and it is all there is when
    // `ps` could not be read.
    let command = devpit_pty::front_on(fronts, &pane.tty)
        .and_then(|front| devpit_pty::agents::program_of(&front.argv))
        .unwrap_or(pane.command);

    let agent = devpit_pty::front_on(fronts, &pane.tty)
        .and_then(|front| devpit_pty::agents::recognise(&front.argv));

    // What devpit started beats what the process looks like, and only here.
    // `glm` and `claude2` are the same binary with the same argv — they differ
    // in environment alone, and telling them apart from outside would mean
    // reading another process's environ, which is where its tokens live.
    // devpit does not have to: it knows because it started it.
    let mine = names.get(&pane.profile);

    PaneRunning {
        pane_id: pane.leaf_id,
        busy: !devpit_pty::agents::idle_shell(&command),
        label: mine
            .cloned()
            .or_else(|| agent.map(|one| one.label.to_owned()))
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
        let knows = shell_knows();
        let off = crate::projects::store()
            .map(|store| crate::agent_choice::disabled(&store))
            .unwrap_or_default();
        let on = |id: &str| !off.iter().any(|one| one == id);
        // The person's own first. They named them, and a menu that buried
        // `GLM` under twelve CLIs would be the menu they stopped using.
        let mine = crate::projects::store()
            .and_then(|store| crate::agent_profiles::all(&store))
            .unwrap_or_default()
            .into_iter()
            .filter(|one| one.mine)
            .map(|one| KnownAgent {
                id: one.id.clone(),
                label: one.label.clone(),
                launch: devpit_agentcli::running::line(&devpit_agentcli::running::runner(&one)),
                // Typed into a terminal, so a name the shell alone knows still
                // counts. Only nothing at all does not.
                installed: one.installed(),
                enabled: on(&one.id),
                homepage: String::new(),
            });
        let built = devpit_pty::agents::KNOWN.iter().map(|one| KnownAgent {
            id: one.id.to_owned(),
            label: one.label.to_owned(),
            launch: one.launch.to_owned(),
            installed: knows.contains(program_of(one.launch)),
            enabled: on(one.id),
            homepage: one.homepage.to_owned(),
        });
        joined(mine.collect(), built)
    })
    .await
    .map_err(|err| RpcError::internal(err.to_string()))
}

/// The person's own first, then every built-in they have not overridden.
///
/// An override keeps the built-in's id, so listing both would be one id on two
/// rows, each starting something different.
fn joined(mine: Vec<KnownAgent>, built: impl Iterator<Item = KnownAgent>) -> Vec<KnownAgent> {
    let taken: Vec<String> = mine.iter().map(|one| one.id.clone()).collect();
    mine.into_iter()
        .chain(built.filter(|one| !taken.contains(&one.id)))
        .collect()
}

/// The program a launch line starts, which is its first word.
fn program_of(launch: &str) -> &str {
    launch.split_whitespace().next().unwrap_or(launch)
}

/// Every command name worth asking the shell about.
///
/// The agents a menu can start, plus the commands profile discovery looks for
/// — `claude2` is in the second list and not the first, and it is exactly the
/// name this whole mechanism exists for.
fn names_to_probe() -> Vec<String> {
    let launches = devpit_pty::agents::KNOWN
        .iter()
        .map(|one| program_of(one.launch).to_owned());
    let discovered = devpit_agentcli::profile::DISCOVERED
        .iter()
        .map(|(command, _, _)| (*command).to_owned());
    // The profiles already declared, so one naming a shell function reads as
    // what it is. Asked once at startup like the rest: a profile added later
    // is probed the next time devpit opens, and until then it reads as
    // missing rather than as a terminal-only name.
    let declared = crate::projects::store()
        .map(|store| crate::agent_profiles::commands(&store))
        .unwrap_or_default();
    let mut names: Vec<String> = launches.chain(discovered).chain(declared).collect();
    names.sort();
    names.dedup();
    names
}

/// Which command names this machine can actually start.
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
/// cheap: measured at 1.6 seconds here, which is fine once and absurd once per
/// name. Warmed at startup by [`warm_installed`], so no menu waits for it.
pub(crate) fn shell_knows() -> &'static std::collections::HashSet<String> {
    static FOUND: std::sync::OnceLock<std::collections::HashSet<String>> =
        std::sync::OnceLock::new();
    FOUND.get_or_init(|| ask_the_shell(&names_to_probe()))
}

/// Starts the probe now, so the first menu finds the answer already there.
pub(crate) fn warm_installed() {
    std::thread::spawn(|| {
        let _ = shell_knows();
    });
}

#[cfg(unix)]
fn ask_the_shell(names: &[String]) -> std::collections::HashSet<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    // Interactive, because that is the only mode that reads the file where a
    // function or an alias would be defined.
    let script = names
        .iter()
        .filter(|name| is_a_bare_name(name))
        .map(|name| format!("command -v {name} >/dev/null 2>&1 && echo {name}"))
        .collect::<Vec<_>>()
        .join("; ");

    let Ok(output) = devpit_pty::host_env::command(&shell)
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
fn ask_the_shell(names: &[String]) -> std::collections::HashSet<String> {
    // Windows has no `-ic`, and the daemon is what fills this seam there.
    // Until then every name is offered and the shell says if it is missing.
    names.iter().cloned().collect()
}

/// Whether a name can be pasted into a shell script as itself.
///
/// The probe builds a line of shell out of these, so anything that is not a
/// plain command name is left out rather than quoted: a name needing quoting
/// is a name no `command -v` was ever going to find, and building the guard
/// instead of the escape keeps the script something a person can read.
fn is_a_bare_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '+'))
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
    // A profile first, then a built-in agent. The same menu offers both, and
    // a profile is the more specific answer when an id is both — which it is
    // for a discovered command, whose id *is* the command.
    let start = to_start(&agent_id)?;

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

    let line = start;
    let lock = state.project_lock(&project_id)?;
    let _guard = lock
        .lock()
        .map_err(|_| RpcError::internal("project session lock"))?;
    let server = crate::sessions::tmux_server()?;
    server
        .send_keys(&target, &line)
        .map_err(crate::sessions::tmux_err)?;
    // After the line, and never instead of it: a pane with the agent running
    // and no label is a cosmetic loss, and a label on a pane where the launch
    // failed is a lie.
    let _ = server.name_pane(&target, &profile_id(&agent_id));
    // Remembered so the pane can start it again if tmux loses the window.
    if let Ok(store) = crate::sessions::store() {
        let _ = store.remember_pane_launch(&project_id, &pane_id, &agent_id);
    }
    Ok(line)
}

/// The profile id to record on a pane, empty for a built-in agent.
pub(crate) fn profile_id(id: &str) -> String {
    match profile_names().contains_key(id) {
        true => id.to_owned(),
        false => String::new(),
    }
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
///
/// The MCP server rides the same way and under the same switch: devpit's
/// tools on the line of the agents devpit starts (`agent_reach`).
fn launch_line(launch: &str, agent: &str, settings_flag: Option<&str>) -> String {
    let mut line = launch.to_owned();
    if let Some(settings) = settings_flag.and_then(|_| hook_settings()) {
        line.push(' ');
        line.push_str(&settings_flag.unwrap_or_default().replace("{}", &settings));
    }
    if let Some(tools) = devpit_tools(agent) {
        line.push(' ');
        line.push_str(&tools);
    }
    line
}

/// The flags that give this agent devpit's MCP tools, if it takes them.
fn devpit_tools(agent: &str) -> Option<String> {
    if !integrated() {
        return None;
    }
    let root = Store::root().ok()?;
    crate::agent_reach::mcp_flags(agent, &crate::agent_reach::exe()?, &root.join("mcp.json"))
}

/// Whether devpit reaches into the agents it starts at all — hooks, tools.
/// Switched off, an agent starts exactly as it would by hand.
fn integrated() -> bool {
    crate::projects::store()
        .map(|store| crate::agent_choice::hooks_on(&store))
        .unwrap_or(true)
}

/// The line that starts this id, whether it names a profile or an agent.
///
/// A profile becomes `NAME='value' program --flags`; a built-in agent is the
/// bare word it always was. Either way the hook flag is appended last, because
/// it is about this launch and not about the account.
pub(crate) fn to_start(id: &str) -> Result<String, RpcError> {
    if let Ok(store) = crate::projects::store() {
        if let Ok(profiles) = crate::agent_profiles::all(&store) {
            if let Some(found) = profiles.iter().find(|one| one.id == id && one.mine) {
                let flag = devpit_pty::agents::known(&found.base).and_then(|one| one.settings_flag);
                let said = devpit_agentcli::running::line(&devpit_agentcli::running::runner(found));
                return Ok(launch_line(&said, &found.base, flag));
            }
        }
    }
    let agent = devpit_pty::agents::known(id).ok_or_else(|| {
        RpcError::new(
            devpit_rpc::ErrorCode::NotFound,
            format!("{id} is not an agent this build knows"),
        )
    })?;
    Ok(launch_line(agent.launch, agent.id, agent.settings_flag))
}

/// Where the hook settings live, written if they are not there yet.
///
/// The same file the board's headless turns use, and written by the same
/// rule: only when it differs, so starting an agent does not touch the disk
/// for nothing.
fn hook_settings() -> Option<String> {
    // Switched off means the flag is never added, so the agent is started
    // exactly as it would have been by hand.
    if !integrated() {
        return None;
    }
    let root = Store::root().ok()?;
    let endpoint = devpit_agentcli::endpoint_file(&root);
    let path = root.join("hooks.json");
    let wanted = devpit_agentcli::settings_json(&endpoint, &devpit_agentcli::auth_file(&root));
    if std::fs::read_to_string(&path).ok().as_deref() != Some(wanted.as_str()) {
        std::fs::create_dir_all(&root).ok()?;
        // Private: these are the commands an agent runs on every hook, so a
        // file someone else can write is a command someone else chose.
        devpit_core::home::write_private(&path, wanted.as_bytes()).ok()?;
    }
    // Quoted, because a home directory with a space in it would otherwise
    // become two arguments to the shell this is typed into.
    Some(format!("'{}'", path.display()))
}

/// Whether a pane is ready to be typed into.
pub(crate) enum Ready {
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

pub(crate) fn settled(session: &str, pane_id: &str) -> Ready {
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

#[cfg(test)]
#[path = "shell_launch_tests.rs"]
mod tests;
