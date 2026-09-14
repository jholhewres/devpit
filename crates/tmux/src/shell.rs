//! The shell a new tmux window starts.
//!
//! tmux runs the login shell with no arguments by default, and a shell started
//! that way says nothing about itself: no prompt boundary, no command start,
//! no exit code. Nothing for [`devpit_pty::osc`] to read, so a terminal is a
//! stream of bytes rather than a sequence of commands that worked or did not.
//!
//! The caller hands the program, its arguments and the environment that turns
//! the markers on; this turns them into the tail of a `new-window` line.

/// The shell a new window starts, and the environment that wraps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shell {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

/// The name a window hands down to everything it starts.
///
/// An agent's hooks fire in a process the app never spawned and cannot see:
/// the shell started the agent, the agent runs the hook. The environment is
/// the only thing that reaches that far, so the leaf id travels in it and
/// comes back on every hook the agent sends.
pub const PANE_ENV: &str = "DEVPIT_PANE";

/// The `-e KEY=VAL … -- program args…` tail.
///
/// With no shell configured it is only the environment, which leaves tmux
/// starting the login shell exactly as it always did — the honest fallback
/// when the startup file could not be written.
///
/// The `--` is load-bearing: without it tmux reads the shell's own flags as
/// its own, and `--rcfile` becomes an error rather than an argument.
pub fn window_args(shell: Option<&Shell>, window: &str) -> Vec<String> {
    let mut args = vec!["-e".to_owned(), format!("{PANE_ENV}={window}")];
    let Some(shell) = shell else {
        // Still worth setting: a person who types `claude` into an unwrapped
        // shell should be as visible as one who picks it from the menu.
        return args;
    };
    for (key, value) in &shell.env {
        args.push("-e".to_owned());
        args.push(format!("{key}={value}"));
    }
    args.push("--".to_owned());
    args.push(shell.program.clone());
    args.extend(shell.args.iter().cloned());
    args
}

/// What is running in a window right now, as far as tmux can say.
///
/// The foreground process, which is the honest answer to "is an agent open in
/// this terminal": the shell asks the kernel, and nothing has to be installed
/// in the person's own configuration to find out.
///
/// tmux names the *executable*, and for an agent CLI that is almost never its
/// own name — Claude Code, Codex, Gemini and OpenCode are all JavaScript, so
/// every one of them reports `node`. Measured. So the tty comes back too: it
/// is the handle for asking the process table what the arguments were, which
/// is where the name actually is. Naming what is in a pane finishes in
/// [`devpit_pty::agents`]; this says where to look.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Running {
    /// The tmux window, which is the app's leaf id.
    pub leaf_id: String,
    /// The foreground executable's own name: `zsh`, `node`, `cargo`.
    pub command: String,
    /// The terminal this pane owns, as tmux writes it: `/dev/pts/8`.
    pub tty: String,
    /// The profile devpit started here, by id — empty when devpit did not
    /// start it, or started a built-in agent.
    ///
    /// Kept in tmux and not in this process, because tmux is the thing that
    /// outlives the app: a pane survives a restart, and so should the answer
    /// to what is in it. An id rather than a name, so renaming the profile
    /// renames the row and a name with a space in it cannot break the format.
    pub profile: String,
}

pub fn parse_running(listed: &str) -> Vec<Running> {
    listed
        .lines()
        .filter_map(|line| {
            let mut parts = line.trim().split(' ');
            let leaf_id = parts.next()?;
            let tty = parts.next()?;
            let command = parts.next()?.trim();
            // Last, and optional: tmux prints nothing at all for a user option
            // that was never set, so a pane devpit did not start is a line one
            // field shorter rather than a line that fails to parse.
            let profile = parts.next().unwrap_or_default().trim();
            if leaf_id.is_empty() || tty.is_empty() || command.is_empty() {
                return None;
            }
            Some(Running {
                leaf_id: leaf_id.to_owned(),
                command: command.to_owned(),
                tty: tty.to_owned(),
                profile: profile.to_owned(),
            })
        })
        .collect()
}

/// One line per pane, asked once for a whole session: a sidebar asks this on a
/// timer, and a process spawn per row would cost more than the answer.
pub(crate) const RUNNING_FORMAT: &str =
    "#{window_name} #{pane_tty} #{pane_current_command} #{@devpit_profile}";
