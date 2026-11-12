//! The daemon: accepts its own user's connections and serves each one's
//! frames against the panes it holds.

use std::collections::HashMap;
use std::io;
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::client::Client;
use crate::frame::{self, Frame, Kind};
use crate::pane::{Asked, Pane};
use crate::socket;

/// How long accepting rests after it fails: out of descriptors, it would fail
/// again at once, round and round on a core.
const ACCEPT_RETRY: Duration = Duration::from_millis(100);

#[derive(Default)]
pub struct Daemon {
    panes: Mutex<HashMap<u32, Arc<Pane>>>,
    /// Clients that sent `Watch`: every pane, including those spawned later.
    observers: Mutex<Vec<Arc<Client>>>,
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
        self.admit(listener.incoming(), own);
    }

    /// Serves each connection `incoming` brings from a peer whose uid is `own`.
    fn admit(self: &Arc<Daemon>, incoming: impl Iterator<Item = io::Result<UnixStream>>, own: u32) {
        for accepted in incoming {
            let Ok(stream) = accepted else {
                std::thread::sleep(ACCEPT_RETRY);
                continue;
            };
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
        if let Ok(mut observers) = self.observers.lock() {
            observers.retain(|one| !Arc::ptr_eq(one, &client));
        }
        for id in self.panes() {
            if let Some(pane) = self.pane(id) {
                pane.unobserve(&client);
            }
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
        if one.kind() == Some(Kind::Watch) {
            return self.watch(client);
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
                if one.bytes.first() == Some(&frame::MIRROR) {
                    pane.mirror(client);
                } else {
                    pane.attach(client);
                }
                attached.retain(|one| !Arc::ptr_eq(one, &pane));
                attached.push(pane);
            }
            Some(Kind::Detach) => {
                pane.detach(client);
                attached.retain(|one| !Arc::ptr_eq(one, &pane));
            }
            Some(Kind::Ack) => match one.bytes.get(..4) {
                Some(b) => pane.ack(
                    client,
                    u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize,
                ),
                None => refuse("a short ack"),
            },
            Some(Kind::History) => match one.bytes.get(..8) {
                Some(b) => {
                    let have = u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
                    let page = pane.older(usize::try_from(have).unwrap_or(usize::MAX));
                    client.send(&Frame::new(pane.id, Kind::Older, page));
                }
                None => refuse("a short history request"),
            },
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
                if let Ok(observers) = self.observers.lock() {
                    for one in observers.iter() {
                        pane.observe(one);
                    }
                }
                panes.insert(id, pane);
            }
            Err(why) => refuse(why),
        }
    }

    fn watch(&self, client: &Arc<Client>) {
        if let Ok(mut observers) = self.observers.lock() {
            if observers.iter().any(|one| Arc::ptr_eq(one, client)) {
                return;
            }
            observers.push(Arc::clone(client));
        }
        for id in self.panes() {
            if let Some(pane) = self.pane(id) {
                pane.observe(client);
            }
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
