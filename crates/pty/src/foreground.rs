//! What is actually in front of a terminal.
//!
//! tmux answers this with `#{pane_current_command}`, and the answer is the
//! executable's own name. For an agent CLI that is almost never its name:
//! Claude Code, Codex, Gemini and OpenCode are all JavaScript, so a pane with
//! any of them open reports `node`. Measured, not assumed — a pane running
//! `claude` reports `node`, and so the sidebar showed nothing at all while a
//! conversation was happening in front of it.
//!
//! The argument vector is what carries the name, so this reads that instead.
//! One `ps` for every pane at once, restricted to the ttys asked about: a
//! process spawn per pane on a two-second timer would cost more than the
//! answer is worth, and `ps -e` on a busy machine returns thousands of lines
//! to find eight.
//!
//! The `+` in the state column is the whole trick: it marks the process group
//! that owns the terminal. Exactly the group whose name a person would give if
//! asked what that terminal is doing.

use std::process::Command;

/// One process holding a terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Front {
    /// As `ps` writes it — `pts/8`, `ttys004` — with no `/dev/`.
    pub tty: String,
    pub pid: u32,
    /// The group that owns the terminal.
    ///
    /// The group and not the process, because ending a terminal's work means
    /// ending all of it: an agent runs tools in children of its own, and
    /// signalling only the one process leaves them orphaned and running.
    pub pgid: u32,
    /// The command and its arguments, as the process was started.
    pub argv: Vec<String>,
    /// Waiting on the terminal rather than working.
    ///
    /// `S` in the state column, which is sleeping — for a shell that means it
    /// is blocked reading the tty, which is what being at a prompt *is*. A
    /// shell still running its startup files is `R`, and typing into one of
    /// those is typing into nothing: measured on this machine, a new tmux
    /// window took 1.3 seconds to reach its prompt, and a line sent before
    /// then was simply lost.
    pub resting: bool,
}

impl Front {
    /// The program's own name, with no path and no login dash.
    ///
    /// A login shell wears `-zsh`, which is not a program called `-zsh`.
    pub fn program(&self) -> &str {
        let first = self.argv.first().map(String::as_str).unwrap_or_default();
        basename(first.strip_prefix('-').unwrap_or(first))
    }
}

/// The last path segment, which is the name a person would use.
pub fn basename(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// What holds each of these terminals, or an empty list if `ps` cannot say.
///
/// Empty rather than an error on purpose: this answers a question the sidebar
/// asks on a timer, and a refusal every two seconds is noise about a fact
/// nobody asked to be told.
pub fn looking(ttys: &[String]) -> Vec<Front> {
    if ttys.is_empty() {
        return Vec::new();
    }
    let Ok(output) = Command::new("ps")
        .args(["-o", "pid=,pgid=,tty=,stat=,args=", "-t", &ttys.join(",")])
        .output()
    else {
        return Vec::new();
    };
    // Non-zero when none of the ttys exist any more, which is ordinary: a
    // pane closes between the listing and the asking.
    parse(&String::from_utf8_lossy(&output.stdout))
}

/// Reads `ps -o pid=,pgid=,tty=,stat=,args=`.
///
/// Only the rows in a foreground process group. Everything else on the tty is
/// the shell that started them and whatever it left running in the
/// background, and neither is what the terminal is doing.
pub fn parse(listed: &str) -> Vec<Front> {
    listed
        .lines()
        .filter_map(|line| {
            let mut parts = line.split_whitespace();
            let pid: u32 = parts.next()?.parse().ok()?;
            let pgid: u32 = parts.next()?.parse().ok()?;
            let tty = parts.next()?;
            let state = parts.next()?;
            if !state.contains('+') {
                return None;
            }
            // Split again rather than rejoining what was split: an argument
            // with runs of spaces in it must come back with them.
            let args = line
                .split_once(state)
                .map(|(_, rest)| rest.trim())
                .unwrap_or_default();
            if args.is_empty() {
                return None;
            }
            Some(Front {
                tty: tty.to_owned(),
                pid,
                pgid,
                argv: args.split_whitespace().map(ToOwned::to_owned).collect(),
                resting: state.starts_with('S'),
            })
        })
        .collect()
}

/// What holds one terminal, when several rows share it.
///
/// The leader of the foreground group, which is the thing that was started —
/// everything else in the group is something it went on to spawn.
///
/// This used to take the highest pid, on the reasoning that the last process
/// to start is the one in front. It is not, and an agent is exactly where it
/// breaks: Claude Code runs its MCP servers as children of itself, in its own
/// group, and children start later and so wear higher pids. Measured on this
/// machine, a pane with Claude Code open listed five rows and the highest pid
/// was `bitbucket-mcp` — which is what the sidebar showed the pane was doing.
///
/// The leader is not always on the tty. A pipeline started in the background
/// is led by a subshell that has already gone, and then no row has
/// `pid == pgid` at all — measured, not assumed. So the fallback is the lowest
/// pid, which is the closest thing to "started first" that costs no extra
/// column of `ps`.
pub fn on<'a>(fronts: &'a [Front], tty: &str) -> Option<&'a Front> {
    let here = || all_on(fronts, tty);
    here()
        .find(|front| front.pid == front.pgid)
        .or_else(|| here().min_by_key(|front| front.pid))
}

/// Everything on this terminal, not only what is in front of it.
///
/// The prefix is handled here and nowhere else, which is the point of it
/// existing: `ps` writes `pts/7` and tmux writes `/dev/pts/7`, so comparing
/// the two strings directly matches nothing at all. Written out by hand once,
/// it was a resource monitor reporting every terminal as costing zero — no
/// error, no empty list, just a number that was always right about nothing.
pub fn all_on<'a>(
    fronts: &'a [Front],
    tty: &str,
) -> impl Iterator<Item = &'a Front> + Clone + use<'a> {
    let bare = tty.strip_prefix("/dev/").unwrap_or(tty).to_owned();
    fronts.iter().filter(move |front| front.tty == bare)
}

/// Whether this is a shell waiting for a person.
///
/// Both halves matter. A shell that is not resting is still running its
/// startup files, and a resting process that is not a shell is an agent
/// already open. Only the first is something to type into.
pub fn at_a_prompt(front: &Front) -> bool {
    front.resting && crate::agents::idle_shell(front.program())
}

/// How many looks in a row a prompt has to survive to be believed.
///
/// A shell blocked on one slow line of its own startup — a network call in a
/// `.zshrc`, a version manager reading the disk — looks exactly like a shell
/// blocked on the keyboard. It stops looking that way on the next glance, and
/// two glances cost a tenth of a second.
pub const CONFIRMATIONS: u8 = 2;

/// Counts consecutive looks that saw a prompt.
///
/// Here rather than at each call site because there are two of them — the
/// launch path and the test that measures it — and a rule with two copies is
/// a rule the test can pass while the product fails.
#[derive(Debug, Default)]
pub struct Settling {
    seen: u8,
}

impl Settling {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds one look. True once a prompt has held for [`CONFIRMATIONS`].
    pub fn looked(&mut self, at_prompt: bool) -> bool {
        self.seen = if at_prompt { self.seen + 1 } else { 0 };
        self.seen >= CONFIRMATIONS
    }
}

#[cfg(test)]
#[path = "foreground_tests.rs"]
mod tests;
