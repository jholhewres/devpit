//! A wheel over a pane, turned into what tmux does with one.
//!
//! The client draws on xterm's alternate screen, so xterm keeps no history of
//! its own, and with tmux's mouse off a wheel there becomes arrow keys — which
//! a shell echoes as `^[[B` and a program reading its input takes as keys
//! nobody pressed. Turning the mouse on would give tmux every click and drag
//! too, and selecting text to copy is the terminal's, not tmux's.
//!
//! So the wheel is asked for here, the way tmux's own `WheelUpPane` binding
//! answers it: a program on the alternate screen — an editor, a pager — gets
//! arrow keys, as it would in any terminal; anything else scrolls the pane's
//! history in copy mode, which leaves by itself back at the bottom.

use crate::{Server, TmuxError};

/// What the pane is doing, as far as a wheel is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Held {
    /// The program asked for mouse events — Claude Code does, and scrolls
    /// its own conversation with them.
    pub mouse: bool,
    /// ...in SGR encoding, which is how the wheel is written to it.
    pub sgr: bool,
    /// The program is on the alternate screen: an editor, a pager.
    pub alternate: bool,
    /// The pane is already in tmux's history.
    pub in_mode: bool,
}

const ASKED: &str = "#{mouse_any_flag} #{mouse_sgr_flag} #{alternate_on} #{pane_in_mode}";

pub(crate) fn held(line: &str) -> Held {
    let on: Vec<bool> = line.split_whitespace().map(|one| one == "1").collect();
    let at = |i: usize| on.get(i).copied().unwrap_or(false);
    Held {
        mouse: at(0),
        sgr: at(1),
        alternate: at(2),
        in_mode: at(3),
    }
}

/// The tmux command one scroll becomes, or none.
///
/// A program that asked for the mouse gets the wheel as the mouse — it was
/// sent arrow keys once, and Claude Code took them for its prompt's history.
/// One on the alternate screen without the mouse gets arrow keys, as any
/// terminal gives it. Anything else scrolls the pane's history in copy mode.
pub(crate) fn argv(target: &str, lines: i32, state: Held) -> Option<Vec<String>> {
    let n = lines.unsigned_abs() as usize;
    let t = target.to_owned();
    let keys = |text: String| {
        Some(vec![
            "send-keys".into(),
            "-t".into(),
            t.clone(),
            "-l".into(),
            text,
        ])
    };
    let x = |more: &[&str]| {
        let mut argv: Vec<String> = vec!["send-keys".into(), "-t".into(), t.clone(), "-X".into()];
        argv.extend(more.iter().map(|one| (*one).to_owned()));
        Some(argv)
    };
    if lines == 0 {
        return state.in_mode.then(|| x(&["cancel"])).flatten();
    }
    let up = lines < 0;
    if state.in_mode {
        return x(&[
            "-N",
            &n.to_string(),
            if up { "scroll-up" } else { "scroll-down" },
        ]);
    }
    if state.mouse {
        let button = if up { 64 } else { 65 };
        let one = if state.sgr {
            format!("\x1b[<{button};1;1M")
        } else {
            format!("\x1b[M{}!!", char::from(32 + button as u8))
        };
        return keys(one.repeat(n));
    }
    if state.alternate {
        return Some(vec![
            "send-keys".into(),
            "-t".into(),
            t.clone(),
            "-N".into(),
            n.to_string(),
            if up { "Up" } else { "Down" }.into(),
        ]);
    }
    if !up {
        return None;
    }
    let mut argv: Vec<String> = vec![
        "copy-mode".into(),
        "-e".into(),
        "-t".into(),
        t.clone(),
        ";".into(),
    ];
    argv.extend(x(&["-N", &n.to_string(), "scroll-up"]).unwrap_or_default());
    Some(argv)
}

impl Server {
    /// Types `line` at a pane's prompt and runs it: the line cleared, the
    /// screen cleared, the text — pasted, bracketed, when it is several lines
    /// — then Enter. Literal throughout, so no word of it is read as a key.
    pub fn submit(&self, target: &str, line: &str) -> Result<(), TmuxError> {
        self.require(&["send-keys", "-t", target, "C-e", "C-u", "C-l"])?;
        if line.contains('\n') {
            self.require(&["set-buffer", "-b", "devpit-submit", "--", line])?;
            self.require(&[
                "paste-buffer",
                "-p",
                "-d",
                "-b",
                "devpit-submit",
                "-t",
                target,
            ])?;
        } else {
            self.require(&["send-keys", "-t", target, "-l", "--", line])?;
        }
        self.require(&["send-keys", "-t", target, "Enter"])?;
        Ok(())
    }

    /// Has tmux draw every client of this leaf again from scratch.
    ///
    /// A terminal that was rebuilt, or lost its GPU context, holds nothing of
    /// what tmux drew before, and tmux only sends what changes — so it stayed
    /// blank but for the cursor until the program next printed.
    ///
    /// Answers whether there was a client to draw: one still connecting is
    /// not listed yet, and the caller asks again.
    pub fn redraw(&self, session: &str, window: &str) -> Result<bool, TmuxError> {
        let client_session = crate::naming::client_session(session, window);
        let listed = self.require(&[
            "list-clients",
            "-t",
            &client_session,
            "-F",
            "#{client_name}",
        ])?;
        let listed = String::from_utf8_lossy(&listed.stdout).into_owned();
        let clients: Vec<&str> = listed.lines().filter(|one| !one.is_empty()).collect();
        for client in &clients {
            let _ = self.run(&["refresh-client", "-t", client]);
        }
        Ok(!clients.is_empty())
    }

    /// Scrolls `target` by `lines` — negative is up, into the history — or,
    /// with zero, leaves the history for the live screen.
    pub fn scroll(&self, target: &str, lines: i32) -> Result<(), TmuxError> {
        let asked = self.require(&["display-message", "-p", "-t", target, ASKED])?;
        let state = held(&String::from_utf8_lossy(&asked.stdout));
        let Some(argv) = argv(target, lines, state) else {
            return Ok(());
        };
        let args: Vec<&str> = argv.iter().map(String::as_str).collect();
        self.require(&args)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Held = Held {
        mouse: false,
        sgr: false,
        alternate: false,
        in_mode: false,
    };

    #[test]
    fn a_program_that_asked_for_the_mouse_gets_the_wheel_as_the_mouse() {
        let claude = Held {
            mouse: true,
            sgr: true,
            alternate: true,
            ..PLAIN
        };
        let up = argv("s:w", -2, claude).expect("a command");
        assert_eq!(up[3], "-l");
        assert_eq!(up[4], "\x1b[<64;1;1M\x1b[<64;1;1M");
        assert_eq!(
            argv("s:w", 1, claude).expect("a command")[4],
            "\x1b[<65;1;1M"
        );
    }

    #[test]
    fn a_pager_gets_arrow_keys_and_a_shell_scrolls_its_history() {
        let pager = Held {
            alternate: true,
            ..PLAIN
        };
        assert_eq!(argv("s:w", -3, pager).expect("keys")[5], "Up");
        let shell = argv("s:w", -3, PLAIN).expect("copy mode");
        assert_eq!(
            shell.join(" "),
            "copy-mode -e -t s:w ; send-keys -t s:w -X -N 3 scroll-up"
        );
        assert_eq!(
            argv("s:w", 3, PLAIN),
            None,
            "down past the bottom is nothing"
        );
    }

    #[test]
    fn in_the_history_the_wheel_moves_it_and_zero_leaves_it() {
        let reading = Held {
            in_mode: true,
            ..PLAIN
        };
        assert_eq!(
            argv("s:w", 2, reading).expect("scroll").join(" "),
            "send-keys -t s:w -X -N 2 scroll-down"
        );
        assert_eq!(
            argv("s:w", 0, reading).expect("cancel").join(" "),
            "send-keys -t s:w -X cancel"
        );
        assert_eq!(argv("s:w", 0, PLAIN), None);
    }

    #[test]
    fn what_tmux_says_about_the_pane_is_read_in_order() {
        assert_eq!(
            held("1 1 0 0\n"),
            Held {
                mouse: true,
                sgr: true,
                ..PLAIN
            }
        );
    }
}
