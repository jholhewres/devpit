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

/// What one scroll asks tmux to run: up, down, or out of the history.
pub(crate) fn argv(target: &str, lines: i32) -> Vec<String> {
    let n = lines.unsigned_abs().to_string();
    let t = target;
    let command = |parts: &[&str]| parts.join(" ");
    match lines.signum() {
        -1 => vec![
            "if-shell".into(),
            "-F".into(),
            "-t".into(),
            t.into(),
            "#{||:#{alternate_on},#{mouse_any_flag}}".into(),
            command(&["send-keys", "-t", t, "-N", &n, "Up"]),
            command(&[
                "copy-mode",
                "-e",
                "-t",
                t,
                ";",
                "send-keys",
                "-t",
                t,
                "-X",
                "-N",
                &n,
                "scroll-up",
            ]),
        ],
        1 => vec![
            "if-shell".into(),
            "-F".into(),
            "-t".into(),
            t.into(),
            "#{pane_in_mode}".into(),
            command(&["send-keys", "-t", t, "-X", "-N", &n, "scroll-down"]),
            command(&[
                "if-shell",
                "-F",
                "-t",
                t,
                "'#{alternate_on}'",
                &format!("'send-keys -t {t} -N {n} Down'"),
            ]),
        ],
        _ => vec![
            "if-shell".into(),
            "-F".into(),
            "-t".into(),
            t.into(),
            "#{pane_in_mode}".into(),
            command(&["send-keys", "-t", t, "-X", "cancel"]),
        ],
    }
}

impl Server {
    /// Has tmux draw every client of this leaf again from scratch.
    ///
    /// A terminal that was rebuilt, or lost its GPU context, holds nothing of
    /// what tmux drew before, and tmux only sends what changes — so it stayed
    /// blank but for the cursor until the program next printed.
    pub fn redraw(&self, session: &str, window: &str) -> Result<(), TmuxError> {
        let client_session = crate::naming::client_session(session, window);
        let listed = self.require(&[
            "list-clients",
            "-t",
            &client_session,
            "-F",
            "#{client_name}",
        ])?;
        for client in String::from_utf8_lossy(&listed.stdout)
            .lines()
            .filter(|one| !one.is_empty())
        {
            let _ = self.run(&["refresh-client", "-t", client]);
        }
        Ok(())
    }

    /// Scrolls `target` by `lines` — negative is up, into the history — or,
    /// with zero, leaves the history for the live screen.
    pub fn scroll(&self, target: &str, lines: i32) -> Result<(), TmuxError> {
        let argv = argv(target, lines);
        let args: Vec<&str> = argv.iter().map(String::as_str).collect();
        self.require(&args)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn up_scrolls_the_history_unless_a_program_holds_the_screen() {
        let up = argv("s:w", -3);
        assert_eq!(up[4], "#{||:#{alternate_on},#{mouse_any_flag}}");
        assert_eq!(up[5], "send-keys -t s:w -N 3 Up");
        assert_eq!(
            up[6],
            "copy-mode -e -t s:w ; send-keys -t s:w -X -N 3 scroll-up"
        );
    }

    #[test]
    fn zero_leaves_the_history() {
        assert_eq!(argv("s:w", 0)[5], "send-keys -t s:w -X cancel");
    }
}
