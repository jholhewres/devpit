//! The daemon: accepts its own user's connections and serves each one's
//! frames against the panes it holds.

use std::collections::HashMap;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};

use crate::client::Client;
use crate::frame::{self, Frame, Kind};
use crate::pane::{Asked, Pane};
use crate::socket;

#[derive(Default)]
pub struct Daemon {
    panes: Mutex<HashMap<u32, Arc<Pane>>>,
}

impl Daemon {
    pub fn new() -> Arc<Daemon> {
        Arc::new(Daemon::default())
    }

    /// Serves `listener` until it fails. One thread per connection: a client
    /// is a window, and there are a handful.
    pub fn serve(self: &Arc<Daemon>, listener: UnixListener) {
        self.serve_as(listener, socket::own_uid());
    }

    /// [`Daemon::serve`], admitting only peers whose uid is `own`. A peer that
    /// is anyone else is closed before a byte is read from it.
    pub fn serve_as(self: &Arc<Daemon>, listener: UnixListener, own: u32) {
        for stream in listener.incoming().flatten() {
            let admitted = socket::peer_uid(&stream).is_ok_and(|peer| socket::admitted(peer, own));
            if !admitted {
                continue;
            }
            let daemon = Arc::clone(self);
            std::thread::spawn(move || daemon.connection(stream));
        }
    }

    fn connection(self: &Arc<Daemon>, mut stream: UnixStream) {
        if socket::greet(&mut stream).is_err() {
            return;
        }
        let Ok(writing) = stream.try_clone() else {
            return;
        };
        let client = Client::over(writing);
        let mut attached: Vec<Arc<Pane>> = Vec::new();
        while let Ok(Some(one)) = frame::read(&mut stream) {
            self.handle(&client, &mut attached, one);
        }
        for pane in attached {
            pane.detach(&client);
        }
    }

    /// One frame from `client`. Unknown kinds and unknown panes are refused,
    /// never fatal: the connection carries on.
    pub fn handle(
        self: &Arc<Daemon>,
        client: &Arc<Client>,
        attached: &mut Vec<Arc<Pane>>,
        one: Frame,
    ) {
        let refuse = |why: &str| {
            client.send(&Frame::new(one.pane, Kind::Refused, why.to_owned()));
        };
        if one.kind() == Some(Kind::Spawn) {
            return self.spawn(client, one.pane, &one.bytes);
        }
        let Some(pane) = self.pane(one.pane) else {
            return refuse("no such pane");
        };
        match one.kind() {
            Some(Kind::Input) => {
                if pane.write(&one.bytes).is_err() {
                    refuse("the pane did not take the input");
                }
            }
            Some(Kind::Resize) => match one.bytes.get(..4) {
                Some(b) => {
                    let rows = u16::from_le_bytes([b[0], b[1]]);
                    let cols = u16::from_le_bytes([b[2], b[3]]);
                    if let Err(why) = pane.resize(rows, cols) {
                        refuse(&why);
                    }
                }
                None => refuse("a short resize"),
            },
            Some(Kind::Kill) => pane.kill(),
            Some(Kind::Attach) => {
                pane.attach(client);
                attached.retain(|one| !Arc::ptr_eq(one, &pane));
                attached.push(pane);
            }
            Some(Kind::Detach) => {
                pane.detach(client);
                attached.retain(|one| !Arc::ptr_eq(one, &pane));
            }
            Some(Kind::Ack) => match one.bytes.get(..4) {
                Some(b) => pane.ack(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize),
                None => refuse("a short ack"),
            },
            Some(Kind::History) => {
                let have = one
                    .bytes
                    .get(..8)
                    .map(|b| u64::from_le_bytes(b.try_into().unwrap_or_default()))
                    .unwrap_or(0);
                client.send(&Frame::new(pane.id, Kind::Older, pane.older(have as usize)));
            }
            _ => refuse("not a request"),
        }
    }

    fn spawn(self: &Arc<Daemon>, client: &Arc<Client>, id: u32, bytes: &[u8]) {
        let refuse = |why: String| {
            client.send(&Frame::new(id, Kind::Refused, why));
        };
        let Some(asked) = Asked::parse(bytes) else {
            return refuse("a spawn without size, cwd or command".to_owned());
        };
        let Ok(mut panes) = self.panes.lock() else {
            return refuse("the daemon's lock was poisoned".to_owned());
        };
        if panes.contains_key(&id) {
            return refuse("that pane is already running".to_owned());
        }
        let forget = Arc::downgrade(self);
        match Pane::spawn(id, &asked, move |gone| {
            if let Some(daemon) = forget.upgrade() {
                if let Ok(mut panes) = daemon.panes.lock() {
                    panes.remove(&gone);
                }
            }
        }) {
            Ok(pane) => {
                panes.insert(id, pane);
            }
            Err(why) => refuse(why),
        }
    }

    pub fn pane(&self, id: u32) -> Option<Arc<Pane>> {
        self.panes.lock().ok()?.get(&id).cloned()
    }

    pub fn panes(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self
            .panes
            .lock()
            .map(|panes| panes.keys().copied().collect())
            .unwrap_or_default();
        ids.sort_unstable();
        ids
    }
}

#[cfg(test)]
#[path = "daemon_tests.rs"]
mod tests;
