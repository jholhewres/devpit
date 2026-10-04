//! The terminals a viewer watches: each one a tmux client of its own,
//! running in a pty here, its bytes put on the viewer's queue.

use std::collections::HashMap;
use std::sync::mpsc::Sender;

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine as _;
use devpit_rpc::RemoteOut;
use portable_pty::{CommandBuilder, PtySize};

/// The most terminals one connection watches at once.
const MOST: usize = 4;

/// The panes one connection has open.
#[derive(Default)]
pub(crate) struct Watching {
    viewer: String,
    open: HashMap<String, Open>,
}

struct Open {
    session: String,
    pty: devpit_pty::Session,
}

impl Watching {
    pub(crate) fn new(viewer: u64) -> Watching {
        Watching {
            viewer: viewer.to_string(),
            open: HashMap::new(),
        }
    }

    /// Starts watching `pane` of `project`, typing only when `typing`.
    pub(crate) fn open(
        &mut self,
        project: &str,
        pane: &str,
        typing: bool,
        size: (u16, u16),
        out: Sender<RemoteOut>,
    ) -> Result<(), String> {
        self.close(pane);
        if self.open.len() >= MOST {
            return Err(format!("at most {MOST} terminals at once"));
        }
        let server = crate::sessions::tmux_server().map_err(|err| err.message)?;
        let session = devpit_tmux::Server::session_name(project);
        let running = server.running(&session).map_err(|err| err.to_string())?;
        if !running.iter().any(|one| one.leaf_id == pane) {
            return Err("that terminal is not open".to_owned());
        }
        let argv = server
            .viewer_argv(&session, pane, &self.viewer, typing)
            .map_err(|err| err.to_string())?;
        let mut command = CommandBuilder::new(&argv[0]);
        command.args(&argv[1..]);
        devpit_pty::host_env::scrub_pty(&mut command);
        command.env("TERM", "xterm-256color");
        let size = PtySize {
            cols: size.0.clamp(20, 400),
            rows: size.1.clamp(5, 200),
            pixel_width: 0,
            pixel_height: 0,
        };
        let mut pty = devpit_pty::spawn(command, size).map_err(|err| err.to_string())?;
        let (_, empty) = tokio::sync::mpsc::channel(1);
        let mut frames = std::mem::replace(&mut pty.frames, empty);
        let named = pane.to_owned();
        std::thread::spawn(move || {
            while let Some(bytes) = frames.blocking_recv() {
                let said = RemoteOut::PaneBytes {
                    pane: named.clone(),
                    b64: BASE64.encode(&bytes),
                };
                if out.send(said).is_err() {
                    return;
                }
            }
            let _ = out.send(RemoteOut::PaneClosed { pane: named });
        });
        self.open.insert(pane.to_owned(), Open { session, pty });
        Ok(())
    }

    /// Keys typed into a watched pane. Refused by tmux itself when the
    /// viewer was attached read-only.
    pub(crate) fn input(&mut self, pane: &str, bytes: &[u8]) -> Result<(), String> {
        let open = self
            .open
            .get_mut(pane)
            .ok_or("that terminal is not open here")?;
        open.pty.write(bytes).map_err(|err| err.to_string())
    }

    pub(crate) fn close(&mut self, pane: &str) {
        if let Some(open) = self.open.remove(pane) {
            if let Ok(server) = crate::sessions::tmux_server() {
                server.end_viewer(&open.session, pane, &self.viewer);
            }
            drop(open.pty);
        }
    }

    pub(crate) fn close_all(&mut self) {
        let panes: Vec<String> = self.open.keys().cloned().collect();
        for pane in panes {
            self.close(&pane);
        }
    }
}
