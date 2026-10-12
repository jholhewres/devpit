//! A Noise session over one WebSocket: every binary frame one Noise message,
//! nothing else accepted on it.

use std::io::{ErrorKind, Read, Write};
use std::time::{Duration, Instant};

use tungstenite::{Message, WebSocket};

use crate::noise::{Error, Handshake, Keys, Session};

#[derive(Debug)]
pub enum LinkError {
    /// The handshake or a frame did not open, or a message was past the ceiling.
    Noise(Error),
    /// The other end is not the machine the device paired with.
    Stranger,
    /// The handshake did not finish in time.
    Late,
    /// Something other than a binary frame.
    NotAFrame,
    /// The connection ended.
    Gone,
    /// The relay closed it with a code that says why: 4401 sign in, 4404 the
    /// machine is not connected, 4408 the other end left.
    Closed(u16),
}

impl std::fmt::Display for LinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LinkError::Noise(err) => err.fmt(f),
            LinkError::Stranger => f.write_str("not the machine this device paired with"),
            LinkError::Late => f.write_str("the handshake did not finish in time"),
            LinkError::NotAFrame => f.write_str("something other than a binary frame"),
            LinkError::Gone => f.write_str("the connection ended"),
            LinkError::Closed(code) => write!(f, "the relay closed the connection ({code})"),
        }
    }
}

impl From<Error> for LinkError {
    fn from(err: Error) -> Self {
        LinkError::Noise(err)
    }
}

/// What one read brought.
#[derive(Debug, PartialEq, Eq)]
pub enum Heard {
    /// A whole message.
    Message(Vec<u8>),
    /// A frame of a message still coming, or a ping: the other end is there.
    Alive,
    /// Nothing, within the socket's read timeout.
    Nothing,
}

pub struct Link<S: Read + Write> {
    ws: WebSocket<S>,
    session: Session,
}

/// A close as the relay meant it: its code when it gave one of its own.
fn closed(frame: Option<tungstenite::protocol::CloseFrame>) -> LinkError {
    match frame.map(|frame| u16::from(frame.code)) {
        Some(code) if (4000..5000).contains(&code) => LinkError::Closed(code),
        _ => LinkError::Gone,
    }
}

fn waiting(err: &tungstenite::Error) -> bool {
    matches!(err, tungstenite::Error::Io(io) if matches!(io.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut))
}

fn send_frame<S: Read + Write>(ws: &mut WebSocket<S>, frame: Vec<u8>) -> Result<(), LinkError> {
    ws.send(Message::Binary(frame.into()))
        .map_err(|_| LinkError::Gone)
}

/// The next binary frame before `deadline`.
fn frame_by<S: Read + Write>(
    ws: &mut WebSocket<S>,
    deadline: Instant,
) -> Result<Vec<u8>, LinkError> {
    while Instant::now() < deadline {
        match ws.read() {
            Ok(Message::Binary(frame)) => return Ok(frame.to_vec()),
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => {}
            Ok(Message::Text(_)) => return Err(LinkError::NotAFrame),
            Ok(Message::Close(frame)) => return Err(closed(frame)),
            Err(err) if waiting(&err) => {}
            Err(_) => return Err(LinkError::Gone),
        }
    }
    Err(LinkError::Late)
}

impl<S: Read + Write> Link<S> {
    /// The machine's side: answers the device's handshake within `within`,
    /// taking messages of up to `most` bytes from it. The socket must have a
    /// read timeout, or a silent device holds this forever.
    pub fn answer(
        mut ws: WebSocket<S>,
        keys: &Keys,
        host: &str,
        most: usize,
        within: Duration,
    ) -> Result<Link<S>, LinkError> {
        let deadline = Instant::now() + within;
        let mut handshake = Handshake::responder(keys, host)?;
        while !handshake.finished() {
            if handshake.writes_next() {
                send_frame(&mut ws, handshake.write()?)?;
            } else {
                handshake.read(&frame_by(&mut ws, deadline)?)?;
            }
        }
        Ok(Link {
            ws,
            session: handshake.into_session(most)?,
        })
    }

    /// The device's side: begins the handshake, and goes on only if the
    /// machine proves the key `machine` the device paired with.
    pub fn call(
        mut ws: WebSocket<S>,
        keys: &Keys,
        host: &str,
        machine: &[u8; 32],
        most: usize,
        within: Duration,
    ) -> Result<Link<S>, LinkError> {
        let deadline = Instant::now() + within;
        let mut handshake = Handshake::initiator(keys, host)?;
        while !handshake.finished() {
            if handshake.writes_next() {
                // Before this end says who it is, the other must have.
                if handshake.peer().is_some_and(|peer| &peer != machine) {
                    return Err(LinkError::Stranger);
                }
                send_frame(&mut ws, handshake.write()?)?;
            } else {
                handshake.read(&frame_by(&mut ws, deadline)?)?;
            }
        }
        if handshake.peer().as_ref() != Some(machine) {
            return Err(LinkError::Stranger);
        }
        Ok(Link {
            ws,
            session: handshake.into_session(most)?,
        })
    }

    /// The other end's static key, proven by the handshake.
    pub fn peer(&self) -> [u8; 32] {
        self.session.peer()
    }

    pub fn socket(&self) -> &WebSocket<S> {
        &self.ws
    }

    /// One read: a whole message, a sign of life, or nothing yet.
    pub fn receive(&mut self) -> Result<Heard, LinkError> {
        match self.ws.read() {
            Ok(Message::Binary(frame)) => Ok(match self.session.open(&frame)? {
                Some(message) => Heard::Message(message),
                None => Heard::Alive,
            }),
            Ok(Message::Ping(_) | Message::Pong(_) | Message::Frame(_)) => Ok(Heard::Alive),
            Ok(Message::Text(_)) => Err(LinkError::NotAFrame),
            Ok(Message::Close(frame)) => Err(closed(frame)),
            Err(err) if waiting(&err) => Ok(Heard::Nothing),
            Err(_) => Err(LinkError::Gone),
        }
    }

    /// A message, sealed and sent as its frames.
    pub fn send(&mut self, message: &[u8]) -> Result<(), LinkError> {
        for frame in self.session.seal(message)? {
            send_frame(&mut self.ws, frame)?;
        }
        Ok(())
    }

    /// Ends the session: its keys go with it, and the socket is closed.
    pub fn close(mut self) {
        let _ = self.ws.close(None);
        let _ = self.ws.flush();
    }
}
