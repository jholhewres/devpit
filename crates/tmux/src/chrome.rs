//! What tmux is told not to draw.
//!
//! Set on the server rather than per session, and that is the fix rather than
//! a shortcut: a grouped session does not inherit another session's options,
//! so setting them on the group left the client session — the one a pane
//! actually attaches to — with a green tmux bar along its bottom. The server
//! is ours, on our own socket, so a global here reaches every session,
//! including the ones made later.

pub(crate) const QUIET: [[&str; 2]; 6] = [
    // A status bar inside a pane we already chrome is noise, and it steals a
    // row from the agent's TUI.
    ["status", "off"],
    // Otherwise a shell's title escape renames the window under us, and the
    // layout is keyed by window name.
    ["allow-rename", "off"],
    ["automatic-rename", "off"],
    // A message that hangs around covers the last line of output.
    ["display-time", "1500"],
    // The default half-second swallows an Escape meant for the program inside,
    // which is most of them.
    ["escape-time", "10"],
    // Without this tmux eats every escape sequence it does not itself
    // understand, OSC 133 among them — so a shell that reports its prompt
    // boundaries reports them to tmux and to nobody else. Measured: a plain
    // `OSC 133;A` never reaches the attached client, and the same sequence
    // wrapped in tmux's own passthrough always does.
    ["allow-passthrough", "on"],
];

/// Every option above, as one tmux invocation.
///
/// tmux takes several commands on a line separated by a bare `;`, so setting
/// six options costs one process rather than six. It matters where it runs:
/// the first time a project opens a terminal, while somebody is watching an
/// empty pane.
pub(crate) fn one_line() -> Vec<&'static str> {
    let mut argv = Vec::with_capacity(QUIET.len() * 4);
    for (at, [name, value]) in QUIET.iter().enumerate() {
        if at > 0 {
            argv.push(";");
        }
        argv.extend(["set-option", "-g", name, value]);
    }
    argv
}
